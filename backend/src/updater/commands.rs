//! 更新管线的 Tauri 命令（任务 04）
//!
//! - `check_update`：检查更新（防重入；结果存入共享状态供下载使用）
//! - `download_and_install_update`：下载（进度经 Channel 上报）→ 验签 → 安装
//!
//! 事件时序与 tauri-plugin-updater v2 的 `download_and_install` 命令完全一致
//! （commands.rs:194-209）：`Started{contentLength}`（首个 chunk）→
//! `Progress{chunkLength}`（每个 chunk）→ `Finished`（下载完成、验签前），
//! 前端（任务 07）可直接复用插件的进度逻辑。

use crate::config::app_paths::{current_mode, AppMode};
use crate::service::update_service::{
    download_and_install as service_download_and_install, UpdateCheckResult,
};
use crate::state::http_client;
use crate::updater::download::DownloadProgress;
use crate::updater::state::UpdaterState;
use serde::Serialize;
use tauri::{AppHandle, Manager};

/// 下载进度事件（与插件 `DownloadEvent` 定义一致，前端可复用）
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum DownloadEvent {
    #[serde(rename_all = "camelCase")]
    Started {
        content_length: Option<u64>,
    },
    #[serde(rename_all = "camelCase")]
    Progress {
        chunk_length: u64,
    },
    Finished,
}

/// 检查是否有可用更新
///
/// 返回 `Ok(None)` 表示无可用更新（前端契约 `{update: UpdateInfo | null} | null`
/// 中的 `null`）；有更新时返回 `Some(UpdateCheckResult{update: Some(...)})`，
/// 同时把结果存入共享状态，供 `download_and_install_update` 使用。
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateCheckResult>, String> {
    let state = app.state::<UpdaterState>();
    let _guard = UpdaterState::try_enter(state.inner())?;

    let update = crate::service::update_service::check(&app)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(info) = &update {
        state
            .set_last_check(Some(info.clone()))
            .map_err(|e| e.to_string())?;
    }

    Ok(update.map(|info| UpdateCheckResult { update: Some(info) }))
}

/// 下载并安装更新（下载进度经 `on_event` 上报；成功路径进程退出）
///
/// 流程：取最近一次 check 结果 → 流式下载（发 Started/Progress/Finished）→
/// 验签 → 按运行模式分派安装。安装成功后安装器 / Go updater 接管，
/// 本进程 `exit(0)`；验签或安装失败返回错误（进程保持存活，前端可提示）。
#[tauri::command]
pub async fn download_and_install_update(
    app: AppHandle,
    on_event: tauri::ipc::Channel<DownloadEvent>,
) -> Result<(), String> {
    // 开发模式不更新（check 阶段已返回 None，这里兜底防御直接调用）
    if matches!(current_mode(), AppMode::Develop) {
        return Err("开发模式不更新".to_string());
    }

    let state = app.state::<UpdaterState>();
    let _guard = UpdaterState::try_enter(state.inner())?;

    // 1. 取最近一次 check 的 UpdateInfo（无则要求先 check）
    let info = state
        .last_check()
        .ok_or_else(|| "尚未检查更新，请先执行 check_update".to_string())?;

    // 2. 流式下载（事件时序与插件 download_and_install 命令一致）
    let mut first_chunk = true;
    let mut last_downloaded = 0u64;
    let bytes = crate::updater::download::download(
        http_client::get_client(),
        &info.artifact,
        |p: DownloadProgress| {
            if first_chunk {
                first_chunk = false;
                let _ = on_event.send(DownloadEvent::Started {
                    content_length: p.total,
                });
            }
            // DownloadProgress 只有累计值，本块大小 = 与上次的差值
            let chunk_length = p.downloaded - last_downloaded;
            last_downloaded = p.downloaded;
            let _ = on_event.send(DownloadEvent::Progress { chunk_length });
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    let _ = on_event.send(DownloadEvent::Finished);

    // 3. 验签 + 安装（成功路径 exit(0)，进程退出；失败返回 Err）
    service_download_and_install(&app, &info, bytes, |_| {})
        .await
        .map_err(|e| e.to_string())
}
