//! 更新相关 Tauri 命令（薄壳：取共享状态传给纯编排，不持状态逻辑）
//!
//! 命令名从简（本模块内无同名冲突）：`check` / `download` / `cancel` / `install` / `state`。
//!
//! # 对外契约（统一）
//!
//! 命令统一返回裁剪的展示视图 [`UpdateView`]（tagged union）；业务失败也以
//! `Error` 变体返回，前端 store 无需 try/catch 分支——
//! `Err` 仅用于防重入等入口拒绝（状态未变更，前端可直接提示）。
//!
//! # 事件通道语义（两路隔离，防陈旧帧混淆）
//!
//! - [`UPDATE_VIEW_EVENT`]（view 通道）：只载**状态迁移帧**（含进入 Downloading 这一次）
//!   与各命令**终态**——同通道内 emit 有序，迁移帧不可能迟到于终态；
//! - [`DOWNLOAD_PROGRESS_EVENT`]（download 通道）：只载**进度窄帧**
//!   （[`DownloadProgress`]，不带 info / severity / 状态）。进度帧迟到不会把
//!   UI 从终态打回下载中——拆通道即隔离陈旧帧。
//!
//! 命令返回与广播共用同一类型与同一 apply 逻辑；进度帧是独立窄类型。

use crate::common::entity::update::DownloadProgress;
use crate::common::enums::update::UpdateView;
use crate::service::update as update_service;
use crate::state::update::UpdaterState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

/// 展示视图广播事件名：状态迁移帧（含进入 Downloading）与各命令终态均走此通道
pub const UPDATE_VIEW_EVENT: &str = "rollcaller://update/view";

/// 下载进度窄帧事件名：仅 Downloading 期的进度帧（`{downloaded, total}`）
pub const DOWNLOAD_PROGRESS_EVENT: &str = "rollcaller://update/download";

/// 广播载荷到所有窗口（失败静默：前端随后可经命令返回值校准）
fn broadcast<T: Serialize>(event: &str, app: &AppHandle, payload: &T) {
    let _ = app.emit(event, payload);
}

/// 检查是否有可用更新（返回最新展示视图；终态走 view 通道）
#[tauri::command]
pub async fn check(app: AppHandle) -> Result<UpdateView, String> {
    let state = app.state::<UpdaterState>();
    let current_version = app.package_info().version.clone();
    let view = update_service::check(state.inner(), &current_version)
        .await
        .map_err(|e| e.to_string())?;
    broadcast(UPDATE_VIEW_EVENT, &app, &view);
    Ok(view)
}

/// 下载已批准产物
///
/// 信号分两路注入编排层：进入下载的**迁移帧**走 view 通道，下载过程经节流的
/// **进度窄帧**走 download 通道；命令返回的终态视图由本命令再广播到 view 通道。
#[tauri::command]
pub async fn download(app: AppHandle) -> Result<UpdateView, String> {
    let state = app.state::<UpdaterState>();
    let current_version = app.package_info().version.clone();
    let view = update_service::download(
        state.inner(),
        &current_version,
        |v| broadcast(UPDATE_VIEW_EVENT, &app, v),
        |p| broadcast(DOWNLOAD_PROGRESS_EVENT, &app, p),
    )
        .await?;
    broadcast(UPDATE_VIEW_EVENT, &app, &view);
    Ok(view)
}

/// 取消下载 / 放弃已下载产物（终态走 view 通道）
#[tauri::command]
pub async fn cancel(app: AppHandle) -> Result<UpdateView, String> {
    let state = app.state::<UpdaterState>();
    let view = update_service::cancel(state.inner())?;
    broadcast(UPDATE_VIEW_EVENT, &app, &view);
    Ok(view)
}

/// 安装已下载产物（成功路径进程退出，安装器接管；失败返回 Error 视图）
#[tauri::command]
pub async fn install(app: AppHandle) -> Result<UpdateView, String> {
    let state = app.state::<UpdaterState>();
    let current_version = app.package_info().version.clone();
    let view = update_service::install(state.inner(), &current_version).await?;
    broadcast(UPDATE_VIEW_EVENT, &app, &view);
    Ok(view)
}

/// 查询当前展示视图（页面挂载初始化用，无网络、无副作用）
#[tauri::command]
pub async fn state(app: AppHandle) -> Result<UpdateView, String> {
    Ok(app.state::<UpdaterState>().view())
}
