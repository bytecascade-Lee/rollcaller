//! 退出前清理钩子注册表
//!
//! 全局注册"进程退出前需要清理的资源"，在确认即将退出时统一执行：
//! - **注册**：任何模块在启动期（如 [`crate::bootstrap`]）调用 [`register`]，注册
//!   async 闭包（如关闭数据库连接池）；
//! - **执行**：更新安装链路 spawn 安装器成功后、`exit(0)` 之前调用 [`run_all`]——
//!   只有"确认即将退出"才执行，失败路径（安装器启动失败、进程继续存活可重试）
//!   不触发，保证清理过的资源（如已关闭的数据库）不会被仍在运行的进程继续使用。
//!
//! # 语义
//!
//! - 一次性：`run_all` 取出并清空全部钩子（退出只发生一次）；
//! - async 友好：钩子为 `FnOnce() -> Future`，逐个 `await`（数据库关闭等均为 async）；
//! - 容错：单个钩子失败只记录日志，不中断后续钩子、不阻塞退出；
//! - 错误处理在注册闭包内完成（返回 `Output = ()`）。

use std::future::Future;
use std::pin::Pin;
use std::sync::LazyLock;
use std::sync::Mutex;

/// 一个退出前钩子：无参 async 闭包，错误在闭包内部记录
type AsyncHook = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

static HOOKS: LazyLock<Mutex<Vec<AsyncHook>>> = LazyLock::new(|| Mutex::new(Vec::new()));

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
    slots.push(Box::new(move || Box::pin(hook())));
}

/// 依次执行全部已注册钩子（取空列表，一次性）；单个失败仅记录日志
pub async fn run_all() {
    let hooks = {
        let mut slots = HOOKS.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *slots)
    };
    for hook in hooks {
        hook().await;
    }
}
