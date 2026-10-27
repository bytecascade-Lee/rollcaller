//! 退出前清理钩子注册表 + **全进程唯一的退出出口**
//!
//! 只有「确认即将退出」才走这里：[`finish_and_exit`] 先跑完清理钩子（如关闭数据库连接池），
//! 再 `exit(code)`，永不返回。安装器启动失败、进程继续存活可重试这类路径**不能**调用它——
//! 否则清理过的资源（如已关闭的数据库）会被仍在运行的进程继续使用。
//!
//! # 契约
//!
//! - **注册**：启动期（见 [`crate::bootstrap`]）调用 [`register`]，注册 async 闭包；
//! - **顺序**：注册顺序即执行顺序；需要排最后的（如 flush 日志）最后注册；
//! - **一次性**：钩子被取出后列表即清空，`run_all` 只会真正跑一次；
//! - **panic 隔离**：每个钩子跑在独立 task 上，panic 只会让这一条失败并记 error，
//!   既不打断后续钩子，也不会把 `exit(code)` 一起带走（否则进程会在该退出时赖着不走）；
//! - **看门狗**：钩子卡死（等一把永远放不掉的锁）时，[`EXIT_WATCHDOG`] 后无条件退出；
//! - **注册太晚**：退出流程开始后再 [`register`] 会被忽略并记一条 warning——
//!   此时没人会再跑钩子，静默吞掉等于资源泄漏且没有痕迹。
//!
//! # 调用点
//!
//! 1. **更新安装链路**：安装器已 spawn，本进程必须让位（`service/update.rs`）；
//! 2. **控制台中断**：终端里按 Ctrl+C（`console.rs`，Windows 专属）。

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

/// 看门狗时限：钩子总时长超过它就直接退出，不让一个出问题的清理步骤拖住整个进程
const EXIT_WATCHDOG: Duration = Duration::from_secs(2);

/// 一个退出前钩子：无参 async 闭包，错误在闭包内部记录
type AsyncHook = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

static HOOKS: LazyLock<Mutex<Vec<AsyncHook>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// 退出流程是否已开始（与 [`HOOKS`] 共用同一把锁读写，见 `run_all`）
static DONE: AtomicBool = AtomicBool::new(false);

/// 注册一个退出前清理钩子（async 闭包；错误自行记录）
///
/// 典型用法（启动期注册一次，见 `bootstrap::register_exit_hooks`）：
///
/// ```ignore
/// shutdown_hooks::register(|| async {
///     if let Err(e) = crate::database::database_pool::close().await {
///         tracing::error!("关闭数据库失败：{e}");
///     }
/// });
/// ```
pub fn register<F, Fut>(hook: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let mut slots = HOOKS.lock().unwrap_or_else(|e| e.into_inner());
    if DONE.load(Ordering::SeqCst) {
        tracing::warn!("退出流程已开始，忽略后注册的清理钩子：对应资源不会被清理");
        return;
    }
    slots.push(Box::new(move || Box::pin(hook())));
}

/// 依次执行全部已注册钩子（取空列表，一次性）
///
/// 每个钩子单独成 task：`JoinHandle` 在 panic / 取消时返回 `Err`，所以一条钩子炸了
/// 不影响后面的钩子，也不会让调用方的 `exit(code)` 永远到不了。
async fn run_all() {
    let hooks = {
        let mut slots = HOOKS.lock().unwrap_or_else(|e| e.into_inner());
        // 与 register 共用同一把锁：要么这次注册被本次退出看见，要么被 register 明确忽略
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

/// 唯一的退出出口：跑完清理钩子后 `exit(code)`，**永不返回**
///
/// 只在「确认即将退出」时调用（安装器已 spawn / 收到控制台中断）。
pub async fn finish_and_exit(code: i32) -> ! {
    // 看门狗：正常路径几毫秒就走完，这个线程随进程退出一起消失；
    // 只有钩子卡住时它才生效——那时已经没别的办法把进程弄出去
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
