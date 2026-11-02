pub mod app_window;
pub mod common;
pub mod help_window;

use crate::windows::common::WindowKind;
use tauri::Manager;

/// 打开窗口
///
/// 返回值表示**本次是否新建**。调用方（`cmd::nav`）要据此决定"发不发唤醒事件"：
/// 新建的窗口目标前端还没开始监听，事件必丢——载荷留在 pending 里等它自己来取。
pub fn open_by_host(app: &tauri::AppHandle, kind: WindowKind) -> anyhow::Result<bool> {
    let created = app.get_webview_window(kind.as_str()).is_none();
    match kind {
        WindowKind::App => app_window::open(app)?,
        WindowKind::Help => help_window::open(app)?,
    }
    Ok(created)
}
