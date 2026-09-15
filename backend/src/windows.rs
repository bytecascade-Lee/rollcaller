pub mod app_window;
pub mod help_window;

use tauri::Manager;

/// 按窗口名（= 路由 host）打开窗口：不存在则建窗，已存在则唤到前台
///
/// 返回值表示**本次是否新建**。调用方（`cmd::nav`）要据此决定"发不发唤醒事件"：
/// 新建的窗口目标前端还没开始监听，事件必丢——载荷留在 pending 里等它自己来取。
///
/// 这张"窗口名 → 建窗函数"的小表是全局唯一的映射，而且**必需**：域可以等于窗口名
/// （所以不需要 域→窗口 表），但"窗口名 → 怎么建"总得有人知道。
pub fn open_by_host(app: &tauri::AppHandle, host: &str) -> anyhow::Result<bool> {
    let created = app.get_webview_window(host).is_none();
    if host == app_window::NAME {
        app_window::open(app.clone())?;
    } else if host == help_window::NAME {
        help_window::open(app.clone())?;
    } else {
        anyhow::bail!("unknown window host: {host}");
    }
    Ok(created)
}
