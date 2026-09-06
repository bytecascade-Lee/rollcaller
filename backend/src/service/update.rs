//! 更新编排服务：领域子模块聚合 + check / download / cancel / install 编排
//!
//! # 分层
//!
//! - 领域子模块（`version` / `check` / `verify` / `download` / `install`）：纯逻辑，
//!   不感知状态与 Tauri；
//! - 本文件顶层编排函数：读 [`UpdaterState`] 会话 → 按阶段守卫 → 调领域实现 →
//!   原子推进状态 → 返回投影快照 [`UpdateState`]。
//!
//! # 状态机约定（编排层负责推进）
//!
//! | 阶段 | 可进入的命令 | 下一阶段 |
//! |---|---|---|
//! | Idle | check | Checking → UpToDate / Available / Error |
//! | Checking | —（防重入） | 同上 |
//! | UpToDate / Available / Downloaded / Error | check | 同上 |
//! | Available / Error(Download) | download | Downloading |
//! | Downloading | cancel（置取消位） | Available（取消）/ Downloaded（成功）/ Error(Download)（失败） |
//! | Downloaded | cancel / install | Available（取消删产物）/ exit(0)（成功）/ Error(Install)（失败） |
//!
//! 错误统一收敛为 `status = Error` + `error_kind`（Check / Download / Install），
//! 前端据此给"重试"按钮；除 Download 取消回 Available 外，其余失败均保留
//! 会话凭据（`artifact` / `downloaded_path`），重试对应命令即可。

pub mod version;
pub mod check;
pub mod verify;
pub mod download;
pub mod install;

use crate::common::entity::update::{Artifact, Policy, UpdateState};
use crate::common::enums::update::{UpdateDecision, UpdateErrorKind, UpdateSource, UpdateStatus};
use crate::config::app_paths::{current_mode, AppMode};
use crate::service::update::check as updater_check;
use crate::service::update::download::DownloadProgress;
use crate::state::http_client;
use crate::state::update::UpdaterState;
use serde::Serialize;
use std::path::{Path, PathBuf};
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

/// 下载产物根目录：`temp/update/downloads/<version>/`（按版本分目录存放）
///
/// 产物属 app 自管 temp（非系统临时目录），完整安装包/更新包留在此处直到安装或取消；
/// "已下载待安装"以磁盘事实（文件存在且校验通过）表达，与内存会话解耦。
fn downloads_root() -> PathBuf {
    crate::config::app_paths::temp_dir()
        .join("update")
        .join("downloads")
}

/// 目标版本的产物目录
fn artifact_dir(version: &str) -> PathBuf {
    downloads_root().join(version)
}

/// 产物正式文件名 = artifact.url 最后一段（与 download 域的落盘命名约定一致）
fn artifact_file_name(artifact: &Artifact) -> Option<String> {
    artifact
        .url
        .path_segments()
        .and_then(|segments| segments.last())
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
}

/// 目标版本的预期产物完整路径（URL 无文件名时为 `None`，交由 download 域报错）
fn expected_artifact_path(artifact: &Artifact, version: &str) -> Option<PathBuf> {
    artifact_file_name(artifact).map(|name| artifact_dir(version).join(name))
}

/// 就绪锚点判定：产物文件存在 → 先以 size 短路（不符视为损坏清除）→ 再整体验签
///
/// 返回 `Some(路径)` 表示"已下载待安装"（可跳过下载直接安装）；
/// 不存在 / 校验失败均返回 `None`，且校验失败或 size 不符时清除文件待重下。
fn ready_artifact(path: &Path, artifact: &Artifact) -> Option<PathBuf> {
    if !path.is_file() {
        return None;
    }
    let size_ok = path.metadata().map(|m| m.len()).ok() == Some(artifact.size);
    if !size_ok {
        let _ = std::fs::remove_file(path);
        return None;
    }
    match verify::verify_artifact_path(path, artifact) {
        Ok(()) => Some(path.to_path_buf()),
        Err(_) => {
            let _ = std::fs::remove_file(path);
            None
        }
    }
}

/// 当前用户更新策略
///
/// TODO(设置存储)：策略将来自用户设置（level/channel/allow_downgrade）；当前
/// 尚无设置存储，返回出厂默认（Patch + Stable）。接入设置后只需改这一处，
/// check 与 download 复核会自动吃到新策略。
pub fn current_policy() -> Policy {
    Policy::default()
}

/// 检查是否有可用更新（编排）
///
/// 阶段守卫：`Checking` / `Downloading` 拒绝重入。结果一律以快照返回：
/// 命中 → 目标产物已在磁盘且校验通过则 `Downloaded`（恢复现场），否则
/// `Available`（severity=critical 即强制更新）；无更新 → `UpToDate`；
/// 失败 → `Error(Check)`（保留原会话内容，供重试后覆盖）。
pub async fn check_update(app: &AppHandle) -> Result<UpdateState, String> {
    let state = app.state::<UpdaterState>();

    // 入口守卫并置 Checking（此刻起防重入）
    state.mutate(|s| {
        if matches!(s.status, UpdateStatus::Checking | UpdateStatus::Downloading) {
            return Err("更新操作正在进行中，请稍候再试".to_string());
        }
        s.status = UpdateStatus::Checking;
        s.error = None;
        s.error_kind = None;
        Ok(())
    })?;

    let policy = current_policy();
    let current_version = app.package_info().version.clone();
    let mode = current_mode();
    let cache_dir = crate::config::app_paths::cache_dir();
    let outcome = updater_check::check(
        http_client::client(),
        UpdateSource::CNB,
        &current_version,
        &policy,
        mode,
        cache_dir,
    )
        .await;

    // 锁外磁盘探测：以"产物文件存在且校验通过"为已下载判据（不以旧会话记录为准），
    // 命中且就绪 → 恢复为 Downloaded（可跳过下载直接安装）；文件损坏则清除待重下。
    let old = state.session();
    let (ready_path, discarded) = match &outcome {
        Ok(Some(found)) => {
            let ready = expected_artifact_path(&found.artifact, &found.info.version.to_string())
                .and_then(|path| ready_artifact(&path, &found.artifact));
            // 换目标 / 清空时清理旧下载产物（尽力，失败不阻塞）
            let discard = old
                .downloaded_path
                .as_ref()
                .filter(|p| Some((*p).clone()) != ready)
                .cloned();
            (ready, discard)
        }
        Ok(None) => (None, old.downloaded_path.clone()),
        Err(_) => (None, None), // 检查失败：保留原会话内容，不触碰磁盘
    };

    let result = state.mutate(|s| {
        match &outcome {
            // 命中：产物已就绪 → Downloaded（可续装）；否则 Available（可下载）
            Ok(Some(found)) => {
                s.status = if ready_path.is_some() {
                    UpdateStatus::Downloaded
                } else {
                    UpdateStatus::Available
                };
                s.info = Some(found.info.clone());
                s.severity = found.severity;
                s.artifact = Some(found.artifact.clone());
                s.current_version = Some(current_version.clone());
                s.downloaded_path = ready_path;
                s.downloaded = 0;
                s.total = None;
                s.error = None;
                s.error_kind = None;
            }
            // 无更新：清空会话（作废旧凭据与产物）
            Ok(None) => {
                s.status = UpdateStatus::UpToDate;
                s.info = None;
                s.severity = Default::default();
                s.artifact = None;
                s.current_version = None;
                s.downloaded_path = None;
                s.downloaded = 0;
                s.total = None;
                s.error = None;
                s.error_kind = None;
            }
            // 检查失败：保留原会话内容，仅落错误（重试 check 后覆盖）
            Err(e) => {
                s.status = UpdateStatus::Error;
                s.error = Some(e.to_string());
                s.error_kind = Some(UpdateErrorKind::Check);
            }
        }
        Ok(())
    })?;

    // 尽力清理被替换/作废的旧下载产物（失败不阻塞）
    if let Some(path) = discarded {
        let _ = std::fs::remove_file(path);
    }
    Ok(result)
}

/// 下载已批准产物（编排）
///
/// 阶段守卫：`Downloaded` 幂等直接返回；`Checking` / `Downloading` 拒绝；
/// 仅 `Available` 或携带下载凭据的 `Error(Download)` 允许进入。
/// 产物按版本分目录存放（`temp/update/downloads/<version>/`）；下载前先探测
/// 就绪锚点——目标文件已存在则不重复下载，但仍整体验签（校验失败清除重下）。
/// 成功后会话推进 `Downloaded` 并记录产物路径，取消回 `Available`（可重下），
/// 其它失败落 `Error(Download)`（凭据保留）。
/// 入口用最新策略复核凭据。
pub async fn download_update(
    app: &AppHandle,
    on_event: tauri::ipc::Channel<DownloadEvent>,
) -> Result<UpdateState, String> {
    if matches!(current_mode(), AppMode::Develop) {
        return Err("开发模式不更新".to_string());
    }
    let state = app.state::<UpdaterState>();

    // 幂等：已下载完成 → 直接返回（前端可进入"已就绪"）
    if state.session().status == UpdateStatus::Downloaded {
        return Ok(state.snapshot());
    }

    // 入口守卫并置 Downloading
    state.mutate(|s| match s.status {
        UpdateStatus::Checking => Err("检查更新正在进行中，请稍候再试".to_string()),
        UpdateStatus::Downloading => Err("下载已在进行中，请勿重复触发".to_string()),
        UpdateStatus::Available => {
            s.status = UpdateStatus::Downloading;
            Ok(())
        }
        UpdateStatus::Error
        if s.error_kind == Some(UpdateErrorKind::Download) && s.artifact.is_some() =>
            {
                s.status = UpdateStatus::Downloading;
                s.error = None;
                s.error_kind = None;
                Ok(())
            }
        _ => Err("尚未检查到可用更新，请先执行 check".to_string()),
    })?;

    let session = state.session();
    // 入口复核：策略变更 → 旧凭据作废，落 Error(Check) 要求重查
    let decision = match (
        session.current_version.as_ref(),
        session.info.as_ref().map(|i| &i.version),
    ) {
        (Some(current), Some(target)) => updater_check::evaluate(
            current,
            target,
            &current_policy(),
            session.severity,
        ),
        _ => return Err("更新凭据不完整，请重新执行 check".to_string()),
    };
    if decision != UpdateDecision::Update {
        state.mutate(|s| {
            s.status = UpdateStatus::Error;
            s.error = Some("更新设置已变更，请重新执行 check_update".to_string());
            s.error_kind = Some(UpdateErrorKind::Check);
            Ok(())
        })?;
        return Ok(state.snapshot());
    }
    let artifact = session.artifact.clone().ok_or_else(|| "更新凭据不完整，请重新执行 check".to_string())?;
    let version_str = session
        .info
        .as_ref()
        .map(|i| i.version.to_string())
        .ok_or_else(|| "更新凭据不完整，请重新执行 check".to_string())?;
    // 产物按版本分目录：temp/update/downloads/<version>/
    let target_dir = artifact_dir(&version_str);

    // 就绪锚点：目标文件已存在 → 跳过下载，但不能跳过校验；校验失败视为损坏清除重下
    if let Some(path) = expected_artifact_path(&artifact, &version_str)
        .and_then(|p| ready_artifact(&p, &artifact))
    {
        return state
            .mutate(|s| {
                s.status = UpdateStatus::Downloaded;
                s.downloaded_path = Some(path);
                s.downloaded = 0;
                s.total = None;
                s.error = None;
                s.error_kind = None;
                Ok(())
            })
            .map_err(|e| e.to_string());
    }

    // 流式下载到 .part → 校验 → 重命名（事件时序：Started → Progress → Finished）
    let mut first_chunk = true;
    let mut last_downloaded = 0u64;
    let result = download::download(
        &artifact,
        &target_dir,
        state.is_cancelled(),
        |p: DownloadProgress| {
            let _ = state.mutate(|s| {
                s.downloaded = p.downloaded;
                s.total = p.total;
                Ok(())
            });
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

    match result {
        Ok(path) => {
            let _ = on_event.send(DownloadEvent::Finished);
            state
                .mutate(|s| {
                    s.status = UpdateStatus::Downloaded;
                    s.downloaded_path = Some(path);
                    s.downloaded = 0;
                    s.total = None;
                    s.error = None;
                    s.error_kind = None;
                    Ok(())
                })
                .map_err(|e| e.to_string())
        }
        Err(e) => {
            let msg = e.to_string();
            // 用户取消 → 回 Available（可重下），不视为错误
            if msg.contains("CANCELLED") {
                state
                    .mutate(|s| {
                        s.status = UpdateStatus::Available;
                        s.downloaded = 0;
                        s.total = None;
                        s.error = None;
                        s.error_kind = None;
                        Ok(())
                    })
                    .map_err(|e| e.to_string())
            } else {
                state
                    .mutate(|s| {
                        s.status = UpdateStatus::Error;
                        s.error = Some(msg);
                        s.error_kind = Some(UpdateErrorKind::Download);
                        Ok(())
                    })
                    .map_err(|e| e.to_string())
            }
        }
    }
}

/// 取消下载 / 放弃已下载产物（编排）
///
/// - 下载进行中：置位取消标志，download 循环感知后回 `Available`；
/// - 已下载完成（取消晚于完成）：删除产物，会话回 `Available`（保留 info/凭据，可重下）。
pub async fn cancel_update(app: &AppHandle) -> Result<UpdateState, String> {
    let state = app.state::<UpdaterState>();
    let session = state.session();

    // 已下载完成：删产物回可重下态
    if session.status == UpdateStatus::Downloaded {
        if let Some(path) = session.downloaded_path {
            let _ = std::fs::remove_file(&path);
            state.mutate(|s| {
                s.status = UpdateStatus::Available;
                s.downloaded_path = None;
                s.error = None;
                s.error_kind = None;
                Ok(())
            })?;
            return Ok(state.snapshot());
        }
    }
    // 下载中：置取消位，由下载流程收尾回 Available
    if session.status == UpdateStatus::Downloading {
        state.request_cancel();
    }
    Ok(state.snapshot())
}

/// 安装已下载产物（编排；成功路径进程退出，安装器接管）
///
/// 阶段守卫：仅 `Downloaded` 或 `Error(Install)`（重试）允许进入。
/// 读盘 → 整体验签 → 启动安装器（成功即退出）；失败落 `Error(Install)`，
/// 产物路径保留，可重试。
pub async fn install_update(app: &AppHandle) -> Result<UpdateState, String> {
    if matches!(current_mode(), AppMode::Develop) {
        return Err("开发模式不更新".to_string());
    }
    let state = app.state::<UpdaterState>();
    let session = state.session();

    match session.status {
        UpdateStatus::Checking => return Err("检查更新正在进行中，请稍候再试".to_string()),
        UpdateStatus::Downloading => return Err("下载进行中，请先完成下载".to_string()),
        UpdateStatus::Downloaded => {}
        UpdateStatus::Error if session.error_kind == Some(UpdateErrorKind::Install) => {}
        _ => return Err("更新尚未下载完成，请先执行 download".to_string()),
    }
    let path = session
        .downloaded_path
        .clone()
        .ok_or_else(|| "下载产物缺失，请重新下载".to_string())?;
    let artifact = session
        .artifact
        .clone()
        .ok_or_else(|| "更新凭据缺失，请重新 check".to_string())?;
    let current = session
        .current_version
        .clone()
        .ok_or_else(|| "当前版本缺失，请重新 check".to_string())?;
    let target = session
        .info
        .as_ref()
        .map(|i| i.version.clone())
        .ok_or_else(|| "目标版本缺失，请重新 check".to_string())?;

    // 读盘复验（install 阶段仍整体校验一次，兜底盘上文件损坏）
    let bytes = std::fs::read(&path)
        .map_err(|e| format!("读取下载产物失败（{}）：{e}", path.display()))?;
    if let Err(e) = verify::verify_artifact(&bytes, &artifact) {
        drop(bytes);
        state.mutate(|s| {
            s.status = UpdateStatus::Error;
            s.error = Some(format!("下载产物校验失败：{e}"));
            s.error_kind = Some(UpdateErrorKind::Install);
            Ok(())
        })?;
        return Ok(state.snapshot());
    }
    drop(bytes);

    // 启动安装器（成功即接管；失败则进程存活、产物保留可重试）
    if let Err(e) = install::launch(current_mode(), &path, &current, &target) {
        state.mutate(|s| {
            s.status = UpdateStatus::Error;
            s.error = Some(e.to_string());
            s.error_kind = Some(UpdateErrorKind::Install);
            Ok(())
        })?;
        return Ok(state.snapshot());
    }

    // 安装器已接管：统一收尾（执行退出前清理并 exit(0)，永不返回）
    install::finish_and_exit().await
}
