//! 更新管线的 Tauri 命令
//!
//! - `check_update`：检查更新（防重入；service 内按覆盖规则落 State 会话）
//! - `download_update`：下载（进度经 Channel 上报）→ service 会话推进 → 变
//!   "已下载待安装"（`downloaded_path = Some`），前端据此显示"重启并更新"
//! - `cancel_update`：取消下载（下载中置取消标志；已完成后删除产物回可重下）
//! - `install_update`：安装已下载产物（成功路径进程 `exit(0)`，安装器接管）
//!
//! 下载/安装分两步与前端契约一致（参照旧版 Updater 的 `Downloading → Downloaded →
//! 重启并更新` 状态机）：下载按钮触发 `download_update`，"已就绪"后由用户点
//! `install_update` 才真正安装。
//!
//! 事件时序沿用 tauri-plugin-updater v2 的 `download_and_install` 命令：
//! `Started{contentLength}`（首个 chunk）→ `Progress{chunkLength}`（每个 chunk）
//! → `Finished`（下载完成、验签前）。

use crate::common::entity::update::CheckOutcome;
use crate::common::enums::update::UpdateDecision;
use crate::config::app_paths::{current_mode, AppMode};
use crate::service::update::check::evaluate;
use crate::service::update::download::DownloadProgress;
use crate::service::update_service;
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
/// 直接透传 service 组装的对外结果 [`CheckOutcome`]（tagged union，前端按
/// `type` 区分：`NoUpdate` / `Available` / `Mandatory` / `Error`）。会话落 State
/// 在 service 内完成，命令层只做防重入与透传。
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<CheckOutcome, String> {
    let state = app.state::<UpdaterState>();
    let _guard = UpdaterState::try_enter(state.inner())?;

    Ok(update_service::check(&app).await)
}

/// 下载更新（进度经 `on_event` 上报；成功后会话标记"已下载待安装"）
///
/// 流程：取当前会话 → 用实时策略复核（check 与下载之间设置变更时自动拦截）→
/// 流式下载到 `.part`（Started/Progress 上报，每 chunk 检查取消）→ 校验 + 重命名
/// （download 内完成）→ Finished → 会话推进。下载失败返回错误且不留产物（前端可
/// 重试）；下载成功但尚未安装时重复调用直接返回 `Ok(())`（幂等），不会重复下载。
#[tauri::command]
pub async fn download_update(app: AppHandle, on_event: tauri::ipc::Channel<DownloadEvent>) -> Result<(), String> {
    // 开发模式不更新（check 阶段已返回 None，这里兜底防御直接调用）
    if matches!(current_mode(), AppMode::Develop) {
        return Err("开发模式不更新".to_string());
    }

    let state = app.state::<UpdaterState>();
    let _guard = UpdaterState::try_enter(state.inner())?;

    // 1. 取当前会话（check 已批准）；无会话 → 要求先 check
    let Some(session) = state.pending() else {
        return Err("尚未检查更新，请先执行 check_update".to_string());
    };

    // 2. 入口复核：用当前实时策略重判（纯本地、零成本）。check 与 download 之间
    //    用户改了 level/channel → 不再批准，作废会话并要求重查——设置变更后的
    //    旧凭据在此被自动拦截，无需主动失效机制
    let approval = evaluate(
        &session.current_version,
        &session.info.version,
        &update_service::current_policy(),
        session.severity,
        session.force,
    )
        .map_err(|e| e.to_string())?;
    if approval != UpdateDecision::Update {
        let _ = state.set_pending(None);
        return Err("更新设置已变更，请重新执行 check_update".to_string());
    }

    // 3. 幂等：已下载完成（重入/重复点击）→ 直接返回，前端可进入"已就绪"
    if session.downloaded_path.is_some() {
        return Ok(());
    }

    // 4. 流式下载到 .part → 校验 → 重命名（事件时序与插件 download_and_install 命令一致）
    let target_dir = crate::config::app_paths::temp_dir().join("downloads");
    let mut first_chunk = true;
    let mut last_downloaded = 0u64;
    let result = crate::service::update::download::download(
        &session.artifact,
        &target_dir,
        state.cancel_flag(),
        |p: DownloadProgress| {
            if first_chunk {
                first_chunk = false;
                let _ = on_event.send(DownloadEvent::Started { content_length: p.total });
            }
            // DownloadProgress 只有累计值，本块大小 = 与上次的差值
            let chunk_length = p.downloaded - last_downloaded;
            last_downloaded = p.downloaded;
            let _ = on_event.send(DownloadEvent::Progress { chunk_length });
        },
    )
        .await;
    // 无论成功 / 失败 / 取消都复位取消标志
    state.reset_cancel();
    let path = result.map_err(|e| e.to_string())?;
    let _ = on_event.send(DownloadEvent::Finished);

    // 5. 会话推进（下载与校验已在 download 内完成）
    update_service::persist_download(&app, path)
        .await
        .map_err(|e| e.to_string())
}

/// 取消下载（或放弃已下载的产物）
///
/// - 下载进行中：置位取消标志，download 每 chunk 检查后中断（清理 .part，前端回
///   Available 可重下）；
/// - 下载已完成（取消晚于完成）：删除已落盘产物，会话回"已批准未下载"。
///
/// 不占用防重入（只置位 / 删文件，不与真正操作争资源）。
#[tauri::command]
pub async fn cancel_update(app: AppHandle) -> Result<(), String> {
    let state = app.state::<UpdaterState>();

    // 若已有"已下载待安装"产物（取消晚于完成）→ 删除产物，回未下载态
    if let Some(mut session) = state.pending() {
        if let Some(path) = session.downloaded_path.take() {
            let _ = std::fs::remove_file(&path);
            state
                .set_pending(Some(session))
                .map_err(|e| e.to_string())?;
        }
    }
    state.request_cancel();
    Ok(())
}

/// 安装已下载的更新（成功路径进程退出，安装器 / Go updater 接管）
///
/// 消费会话中的落盘产物：读盘 → 验签 → 按运行模式分派安装；失败返回错误
/// （进程保持存活），用户可重试。
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    // 开发模式不更新（兜底防御直接调用）
    if matches!(current_mode(), AppMode::Develop) {
        return Err("开发模式不更新".to_string());
    }

    let state = app.state::<UpdaterState>();
    let _guard = UpdaterState::try_enter(state.inner())?;

    update_service::install_downloaded(&app).await.map_err(|e| e.to_string())
}
