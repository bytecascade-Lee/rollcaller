//! 更新相关 Tauri 命令
//!
//! # 对外契约
//!
//! 命令统一返回裁剪的展示视图 [`UpdateView`]；业务失败也以 `Error` 变体返回。
//!
//! # 事件通道语义
//!
//! - [`UPDATE_VIEW_EVENT`]（view 通道）：状态迁移帧，与各命令终态；
//! - [`DOWNLOAD_PROGRESS_EVENT`]（download 通道）：进度窄帧。

use crate::updater;
use crate::updater::model::UpdateView;
use crate::updater::state::UpdaterState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

/// 展示视图广播事件名：状态迁移帧（含进入 Downloading）与各命令终态均走此通道
pub const UPDATE_VIEW_EVENT: &str = "rollcaller://update/view";

/// 下载进度窄帧事件名：仅 Downloading 期的进度帧（`{downloaded, total}`）
pub const DOWNLOAD_PROGRESS_EVENT: &str = "rollcaller://update/download";

/// 广播载荷到所有窗口（失败静默：前端随后可经命令返回值校准）
fn broadcast<T: Serialize>(event: &str, app: &AppHandle, payload: &T) {
    let _ = app.emit(event, payload);
}

/// 检查是否有可用更新
///
/// 进入 Checking 的迁移帧与终态均走 view 通道
#[tauri::command]
pub async fn check(app: AppHandle, state: State<'_, UpdaterState>) -> Result<UpdateView, String> {
    let current_version = app.package_info().version.clone();
    let view = updater::check(state.inner(), &current_version, |v| broadcast(UPDATE_VIEW_EVENT, &app, v)).await?;
    broadcast(UPDATE_VIEW_EVENT, &app, &view);
    Ok(view)
}

/// 下载已批准产物
///
/// 信号分两路注入编排层：进入下载的**迁移帧**走 view 通道，下载过程经节流的
/// **进度窄帧**走 download 通道；命令返回的终态视图由本命令再广播到 view 通道。
#[tauri::command]
pub async fn download(app: AppHandle, state: State<'_, UpdaterState>) -> Result<UpdateView, String> {
    let current_version = app.package_info().version.clone();
    let view = updater::download(
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
pub async fn cancel(app: AppHandle, state: State<'_, UpdaterState>) -> Result<UpdateView, String> {
    let view = updater::cancel(state.inner())?;
    broadcast(UPDATE_VIEW_EVENT, &app, &view);
    Ok(view)
}

/// 安装已下载产物（成功路径进程退出，安装器接管；失败返回 Error 视图）
#[tauri::command]
pub async fn install(app: AppHandle, state: State<'_, UpdaterState>) -> Result<UpdateView, String> {
    let current_version = app.package_info().version.clone();
    let view = updater::install(state.inner(), &current_version).await?;
    broadcast(UPDATE_VIEW_EVENT, &app, &view);
    Ok(view)
}

/// 查询当前展示视图（页面挂载初始化用，无网络、无副作用）
#[tauri::command]
pub async fn state(state: State<'_, UpdaterState>) -> Result<UpdateView, String> {
    Ok(state.view())
}
