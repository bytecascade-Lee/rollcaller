//! 更新相关 Tauri 命令（薄壳：取共享状态传给纯编排，不持状态逻辑）
//!
//! 命令名从简（本模块内无同名冲突）：`check` / `download` / `cancel` / `install` / `state`。
//!
//! # 对外契约（统一）
//!
//! 命令统一返回裁剪的展示视图 [`UpdateView`]（tagged union）；业务失败也以
//! `Error { message, retry }` 变体返回，前端 store 无需 try/catch 分支——
//! `Err` 仅用于防重入等入口拒绝（状态未变更，前端可直接提示）。
//! 状态提交后经 [`UPDATE_VIEW_EVENT`] 广播给所有窗口，命令返回与广播共用同一类型。

use crate::common::entity::update::UpdateView;
use crate::service::update as update_service;
use crate::state::update::UpdaterState;
use tauri::{AppHandle, Emitter, Manager};

/// 展示视图广播事件名：后端任何状态提交后推送（前端 store 订阅）
pub const UPDATE_VIEW_EVENT: &str = "update://view";

/// 广播当前视图到所有窗口（失败静默：前端随后可经命令返回值校准）
fn broadcast(app: &AppHandle, view: &UpdateView) {
    let _ = app.emit(UPDATE_VIEW_EVENT, view);
}

/// 检查是否有可用更新（返回最新展示视图）
#[tauri::command]
pub async fn check(app: AppHandle) -> Result<UpdateView, String> {
    let state = app.state::<UpdaterState>();
    let current_version = app.package_info().version.clone();
    let view = update_service::check_update(state.inner(), &current_version).await?;
    broadcast(&app, &view);
    Ok(view)
}

/// 下载已批准产物（进度经广播实时上报；返回最终展示视图）
#[tauri::command]
pub async fn download(app: AppHandle) -> Result<UpdateView, String> {
    let state = app.state::<UpdaterState>();
    let view = update_service::download_update(state.inner(), |v| broadcast(&app, v)).await?;
    broadcast(&app, &view);
    Ok(view)
}

/// 取消下载 / 放弃已下载产物（返回最新展示视图）
#[tauri::command]
pub async fn cancel(app: AppHandle) -> Result<UpdateView, String> {
    let state = app.state::<UpdaterState>();
    let view = update_service::cancel_update(state.inner())?;
    broadcast(&app, &view);
    Ok(view)
}

/// 安装已下载产物（成功路径进程退出，安装器接管；失败返回 Error 视图）
#[tauri::command]
pub async fn install(app: AppHandle) -> Result<UpdateView, String> {
    let state = app.state::<UpdaterState>();
    let view = update_service::install_update(state.inner()).await?;
    broadcast(&app, &view);
    Ok(view)
}

/// 查询当前展示视图（页面挂载初始化用，无网络、无副作用）
#[tauri::command]
pub async fn state(app: AppHandle) -> Result<UpdateView, String> {
    Ok(app.state::<UpdaterState>().view())
}
