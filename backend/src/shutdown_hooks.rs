//! 退出前清理钩子注册表、全进程唯一的退出出口
//!
//! # 契约
//!
//! - **注册**：启动期（参考 [`crate::bootstrap`]）调用 [`register`]，注册 async 闭包；
//! - **顺序**：注册顺序即执行顺序；需要排最后的（如 flush 日志）最后注册；
//! - **一次性**：钩子被取出后列表即清空，`run_all` 只会真正跑一次；
//! - **panic 隔离**：每个钩子跑在独立 task 上，panic 只会使这一条失败并记 error；
//! - **看门狗**：钩子卡死时，[`EXIT_WATCHDOG`] 后无条件退出；
//! - **注册太晚**：退出流程开始后再 [`register`] 会被忽略并记一条 warning。

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

/// 看门狗时限，钩子总时长超过后直接退出
const EXIT_WATCHDOG: Duration = Duration::from_secs(2);

/// 一个退出前钩子，无参 async 闭包，错误在闭包内部记录
type AsyncHook = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output=()> + Send>> + Send>;

static HOOKS: LazyLock<Mutex<Vec<AsyncHook>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// 退出流程是否已开始，与 [`HOOKS`] 共用同一把锁读写
static DONE: AtomicBool = AtomicBool::new(false);

/// 注册一个退出前清理钩子
///
/// 要求async 闭包，错误需自行记录
pub fn register<F, Fut>(hook: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output=()> + Send + 'static,
{
    let mut slots = HOOKS.lock().unwrap_or_else(|e| e.into_inner());
    if DONE.load(Ordering::SeqCst) {
        tracing::warn!("退出流程已开始，忽略后注册的清理钩子：对应资源不会被清理");
        return;
    }
    slots.push(Box::new(move || Box::pin(hook())));
}

/// 依次执行全部已注册钩子
async fn run_all() {
    let hooks = {
        let mut slots = HOOKS.lock().unwrap_or_else(|e| e.into_inner());
        // 与 register 共用同一把锁
        DONE.store(true, Ordering::SeqCst);
        std::mem::take(&mut *slots)
    };

    for (index, hook) in hooks.into_iter().enumerate() {
        if let Err(e) = tauri::async_runtime::spawn(hook()).await {
            tracing::error!(
                "退出钩子 #{} 执行失败（panic 或任务被取消），继续执行后续钩子：{e}",
                index + 1
            );
        }
    }
}

/// 唯一的退出出口，跑完清理钩子后 `exit(code)`
pub async fn finish_and_exit(code: i32) -> ! {
    std::thread::spawn(move || {
        std::thread::sleep(EXIT_WATCHDOG);
        std::process::exit(code);
    });

    run_all().await;
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;

    /// 钩子表与 DONE 都是进程级全局状态：用例之间必须串行，且每次先清空
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn setup() -> std::sync::MutexGuard<'static, ()> {
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        HOOKS.lock().unwrap_or_else(|e| e.into_inner()).clear();
        DONE.store(false, Ordering::SeqCst);
        guard
    }

    fn run() {
        tauri::async_runtime::block_on(run_all());
    }

    /// 记录执行顺序的共享缓冲区
    fn log() -> Arc<Mutex<Vec<&'static str>>> {
        Arc::new(Mutex::new(Vec::new()))
    }

    #[test]
    fn runs_in_registration_order() {
        let _guard = setup();
        let log = log();
        for tag in ["第一", "第二", "第三"] {
            let log = Arc::clone(&log);
            register(move || async move { log.lock().unwrap().push(tag) });
        }

        run();
        assert_eq!(*log.lock().unwrap(), ["第一", "第二", "第三"]);
    }

    #[test]
    fn runs_only_once() {
        let _guard = setup();
        let count = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&count);
        register(move || async move {
            counter.fetch_add(1, Ordering::SeqCst);
        });

        run();
        run(); // 第二次：钩子已被取空，什么都不跑
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn panic_in_one_hook_does_not_stop_others() {
        let _guard = setup();
        let log = log();

        register(|| async { panic!("这个钩子故意 panic") });
        let after = Arc::clone(&log);
        register(move || async move { after.lock().unwrap().push("后续钩子") });

        // 关键断言：run_all 自己不能被 panic 带走，且后续钩子仍然执行
        run();
        assert_eq!(*log.lock().unwrap(), ["后续钩子"]);
    }

    #[test]
    fn register_after_run_is_ignored() {
        let _guard = setup();
        let count = Arc::new(AtomicUsize::new(0));
        run(); // 先把退出流程标记为已开始

        let counter = Arc::clone(&count);
        register(move || async move {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        run();

        assert_eq!(count.load(Ordering::SeqCst), 0);
    }
}
