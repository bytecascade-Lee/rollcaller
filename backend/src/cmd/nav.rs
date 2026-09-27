//! 跨窗口导航命令（薄壳：只搬运，不持状态逻辑）
//!
//! 只有两个动作：**投递**（`nav_open`）与**取走**（`nav_take_pending`）。
//!
//! # 后端对地址的理解止于"第一段"
//!
//! `route` 形如 `#/<窗口名>/<段...>`。后端**只把第一段当窗口名**——靠"域即窗口名"
//! 这条隐含规则，它不需要"域 → 窗口"映射表；其余段与 `payload` 全程不透明，后端
//! 既不解析也不转换，所以也不需要镜像一套前端类型。
//!
//! # 就绪
//!
//! 没有单独的 ready 命令。冷启动时窗口不存在、后端**不发事件**，载荷静静躺在 pending
//! 里；目标前端挂载后自己会取一次——**这次取走本身就是就绪信号**（一个动作同时表达了
//! "我起来了"和"把东西给我"），比额外维护一个 ready 标志少一个状态。

use crate::state::nav::{NavState, PendingNav};
use crate::windows;
use serde_json::Value;
use tauri::{Emitter, Manager};

/// 唤醒事件名：**不带载荷**，只是"有新东西，来取"
///
/// 刻意不带载荷：载荷只走 pending 一条通路，于是不可能出现"事件里的载荷"与"取到的载荷"
/// 两份真相；重复唤醒也天然幂等（取过一次再取就是 `None`，空操作）。
pub const NAV_WAKE_EVENT: &str = "rollcaller://nav/wake";

/// 路由前缀，与前端 `$routing/route` 的 `ROUTE_PREFIX` 一致
const ROUTE_PREFIX: &str = "#/";

/// 投递一个地址到目标窗口
///
/// 目标窗口不存在则建窗（此时**不发事件**，前端还没开始监听）；已存在则先唤到前台
/// 再发唤醒事件——否则用户在后台窗口里看到内容变了却毫无感知。
#[tauri::command]
pub async fn nav_open(
    app: tauri::AppHandle,
    route: String,
    payload: Option<Value>,
) -> Result<(), String> {
    let host = host_of(&route)
        .ok_or_else(|| format!("invalid route: {route}"))?
        .to_string();

    let created = windows::open_by_host(&app, &host).map_err(|e| e.to_string())?;
    app.state::<NavState>().put(&host, PendingNav { route, payload });

    if !created {
        let _ = app.emit_to(host.as_str(), NAV_WAKE_EVENT, ());
    }
    Ok(())
}

/// 取走并清空**调用窗口**的待取载荷
///
/// host 从窗口 label 取、不由前端传：窗口名就是它自己的身份，没有传错的可能。
#[tauri::command]
pub async fn nav_take_pending(window: tauri::WebviewWindow) -> Option<PendingNav> {
    let host = window.label().to_string();
    window.app_handle().state::<NavState>().take(&host)
}

/// 从 `#/<host>/...` 里取第一段（后端对 route 的唯一"理解"）
fn host_of(route: &str) -> Option<&str> {
    let rest = route.strip_prefix(ROUTE_PREFIX)?;
    match rest.split('/').next() {
        Some(host) if !host.is_empty() => Some(host),
        _ => None,
    }
}
