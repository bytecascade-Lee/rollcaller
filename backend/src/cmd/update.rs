//! 更新相关 Tauri 命令
//!
//! 统一契约：除 `install` 成功路径进程退出外，均返回当前状态快照
//! [`UpdateState`]（含阶段 + 展示信息），前端 store 直接以返回值覆盖镜像；
//! 出错（含操作失败）也返回 `Ok(快照)`，由快照 `status=Error` + `errorKind`
//! 表达；`Err` 仅用于防重入等入口拒绝（状态未变更）。

use crate::common::entity::update::UpdateState;
use crate::service::update::{self, DownloadEvent};
use crate::state::update::UpdaterState;
use tauri::{AppHandle, Manager};

/// 检查是否有可用更新（返回最新状态快照）
#[tauri::command]
pub async fn update_check(app: AppHandle) -> Result<UpdateState, String> {
    update::check_update(&app).await
}

/// 下载已批准产物（进度经 `onEvent` 通道上报；返回最终状态快照）
#[tauri::command]
pub async fn update_download(
    app: AppHandle,
    on_event: tauri::ipc::Channel<DownloadEvent>,
) -> Result<UpdateState, String> {
    update::download_update(&app, on_event).await
}

/// 取消下载 / 放弃已下载产物（返回最新状态快照）
#[tauri::command]
pub async fn update_cancel_download(app: AppHandle) -> Result<UpdateState, String> {
    update::cancel_update(&app).await
}

/// 安装已下载产物（成功路径进程退出，安装器接管）
#[tauri::command]
pub async fn update_install(app: AppHandle) -> Result<UpdateState, String> {
    update::install_update(&app).await
}

/// 查询当前状态快照（页面挂载初始化用，无网络、无副作用）
#[tauri::command]
pub async fn update_state(app: AppHandle) -> Result<UpdateState, String> {
    Ok(app.state::<UpdaterState>().snapshot())
}
