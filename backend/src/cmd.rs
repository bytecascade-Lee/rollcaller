pub mod app_info;
pub mod app_paths;
pub mod attendance_status;
pub mod export;
pub mod help;
pub mod import;
pub mod nav;
pub mod record;
pub mod rollcall;
pub mod student;
pub mod tts;
pub mod updater;
pub mod windows;

/// IPC 出口错误
///
/// # 为什么在这里
///
/// 命令层是唯一同时知道「哪个操作失败」与「它要显示给用户」的位置。`anyhow` 的
/// `Caused by` 链一旦 `to_string()` 就断开，而前端只保留最外层那一句 —— 完整链必须
/// 趁这一刻落进日志。
///
/// # 只记一次
///
/// `service` 层 `return Err` 时不要记 `error`，一律经由本函数出口，否则一次失败两条
/// 日志。后台路径（更新器 spawn、退出钩子、窗口创建）没有命令出口，只能在原地记。
///
/// # 有意的不对称
///
/// 日志用 `{error:#}` 展开整条链，返回给前端的是 `{error}`，只留最外层，因为弹窗只有一行
pub fn ipc_err(command: &'static str, error: impl Into<anyhow::Error>) -> String {
    let error = error.into();
    tracing::error!(cmd = %command, error = %format!("{error:#}"), "命令执行失败");
    error.to_string()
}

/// IPC 入口拒绝
pub fn ipc_refuse(command: &'static str, message: String) -> String {
    tracing::warn!(cmd = %command, reason = %message, "命令被拒绝");
    message
}
