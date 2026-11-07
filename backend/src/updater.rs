//! 更新编排：check / download / cancel / install 四个命令的纯 Rust 实现。
//!
//! 不依赖 `AppHandle`，事件靠命令层注入的回调发出；对外只返回裁剪视图 [`UpdateView`]。
//! 迁移表、错误收敛规则见 `docs/architect/自动更新架构-三段式管线与发布链路.md` §3。
//!
//! 三条贯穿全文件的约定：产物路径由凭据现推（磁盘是事实源，路径不入会话）；进度只落
//! 原子槽、不进会话；入口拒绝返回 `Err`，业务失败折叠成 `UpdateView::Error`。

use crate::config::app_paths::{current_mode, AppMode};
use crate::state::http_client;
use crate::updater::download::Cancelled;
use model::{DownloadProgress, Policy, Severity, UpdateDecision, UpdateError, UpdateSource, UpdateStatus, UpdateView};
use semver::Version;
use state::UpdaterState;
use std::time::{Duration, Instant};

pub mod check;
pub mod constant;
pub mod decide;
pub mod download;
pub mod install;
pub mod model;
pub mod paths;
pub mod remote;
pub mod state;
pub mod urls;
pub mod verify;

const PROGRESS_BROADCAST_INTERVAL: Duration = Duration::from_millis(100);

/// 当前用户更新策略（TODO(设置存储)：接入用户设置后只需改这一处）
pub fn current_policy() -> Policy {
    Policy::default()
}

/// 检查是否有可用更新
///
/// 命中且产物已就绪 → `Downloaded`（恢复现场）；命中未就绪 → `Available`；
/// 无更新 → `UpToDate`；失败 → `Error(Check)`（保留原会话，重试后覆盖）。
pub async fn check(
    state: &UpdaterState,
    current_version: &Version,
    on_view: impl Fn(&UpdateView),
) -> Result<UpdateView, String> {
    let checking = state.mutate(|s| {
        if matches!(s.status, UpdateStatus::Checking | UpdateStatus::Downloading) {
            return Err("更新操作正在进行中，请稍候再试".to_string());
        }
        s.status = UpdateStatus::Checking;
        s.error = None;
        Ok(())
    })?;
    // 迁移帧：前端据此显示"正在检查更新…"，否则界面会从 Idle 直接跳到终态
    on_view(&checking);

    let policy = current_policy();
    let mode = current_mode();
    let source = match mode {
        AppMode::Develop => UpdateSource::Local,
        AppMode::Portable | AppMode::Install(_) => UpdateSource::CNB,
    };
    let outcome = check::check(http_client::client(), source, current_version, &policy, mode).await;

    // 锁外磁盘探测：命中即恢复为 Downloaded（可跳过下载直接安装），损坏则清除待重下
    let ready = match &outcome {
        Ok(Some(found)) => paths::package_of(&found.artifact)
            .ok()
            .and_then(|path| download::ready(&path, &found.artifact)),
        _ => None,
    };

    let found = match outcome {
        Ok(found) => found,
        Err(e) => return state.fail(UpdateError::Check(e.to_string())),
    };
    state.mutate(|s| {
        match found {
            Some(found) => {
                s.status = if ready.is_some() {
                    UpdateStatus::Downloaded
                } else {
                    UpdateStatus::Available
                };
                s.info = Some(found.info);
                s.severity = found.severity;
                s.artifact = Some(found.artifact);
                s.error = None;
            }
            // 无更新：清空会话（作废旧凭据）
            None => {
                s.status = UpdateStatus::UpToDate;
                s.info = None;
                s.severity = Severity::Normal;
                s.artifact = None;
                s.error = None;
            }
        }
        Ok(())
    })
}

/// 下载已批准产物
///
/// `Downloaded` 幂等返回；`Checking` / `Downloading` 拒绝；仅 `Available` 或
/// `Error(Download)` 且带凭据时允许进入。产物已就绪则跳过下载（不跳过验签）。
pub async fn download(
    state: &UpdaterState,
    current_version: &Version,
    on_view: impl Fn(&UpdateView),
    on_progress: impl Fn(&DownloadProgress),
) -> Result<UpdateView, String> {
    if state.session().status == UpdateStatus::Downloaded {
        return Ok(state.view());
    }

    state.mutate(|s| match s.status {
        UpdateStatus::Checking => Err("检查更新正在进行中，请稍候再试".to_string()),
        UpdateStatus::Downloading => Err("下载已在进行中，请勿重复触发".to_string()),
        UpdateStatus::Available => {
            s.status = UpdateStatus::Downloading;
            Ok(())
        }
        UpdateStatus::Error if matches!(s.error, Some(UpdateError::Download(_))) && s.artifact.is_some() => {
            s.status = UpdateStatus::Downloading;
            s.error = None;
            Ok(())
        }
        _ => Err("尚未检查到可用更新，请先执行 check".to_string()),
    })?;

    let session = state.session();
    // 入口复核：设置变更 → 旧凭据作废，要求重查
    let decision = match session.info.as_ref().map(|i| &i.version) {
        Some(target) => decide::evaluate(current_version, target, &current_policy(), session.severity),
        None => return Err("更新凭据不完整，请重新执行 check".to_string()),
    };
    if decision != UpdateDecision::Update {
        return state.fail(UpdateError::Check("更新设置已变更，请重新执行 check".to_string()));
    }
    let artifact = session
        .artifact
        .clone()
        .ok_or_else(|| "更新凭据不完整，请重新执行 check".to_string())?;
    let final_path = paths::package_of(&artifact).map_err(|e| format!("{e}，请重新执行 check"))?;

    if download::ready(&final_path, &artifact).is_some() {
        return state.enter(UpdateStatus::Downloaded);
    }

    // 确认真正进入下载：先广播迁移帧，再流式下载
    on_view(&state.view());
    let mut last_broadcast = Instant::now();
    let result = download::download(&artifact, state.slot().cancel_flag(), |p: DownloadProgress| {
        state.slot().set_progress(p);
        if last_broadcast.elapsed() >= PROGRESS_BROADCAST_INTERVAL {
            on_progress(&state.slot().snapshot());
            last_broadcast = Instant::now();
        }
    })
    .await;
    state.slot().finish();

    match result {
        Ok(_) => state.enter(UpdateStatus::Downloaded),
        // 用户取消不算错误：回 Available，保留 info 与凭据可重下
        Err(e) if e.downcast_ref::<Cancelled>().is_some() => state.enter(UpdateStatus::Available),
        Err(e) => state.fail(UpdateError::Download(e.to_string())),
    }
}

/// 取消下载 / 放弃已下载产物
///
/// 下载中仅置取消位（由下载流程收尾回 `Available`）；已下载完成则删产物回 `Available`。
pub fn cancel(state: &UpdaterState) -> Result<UpdateView, String> {
    let session = state.session();

    if session.status == UpdateStatus::Downloaded {
        if let Some(artifact) = session.artifact.as_ref() {
            if let Ok(path) = paths::package_of(artifact) {
                let _ = std::fs::remove_file(path);
            }
        }
        return state.enter(UpdateStatus::Available);
    }
    if session.status == UpdateStatus::Downloading {
        state.slot().request_cancel();
    }
    Ok(state.view())
}

/// 安装已下载产物（成功路径进程退出，安装器接管）
///
/// 仅 `Downloaded` 或 `Error(Install)` 可进入。读盘后**再次**整体验签：`Downloaded` 不代表
/// 产物仍有效，缺盘或损坏一律落 `Error(Download)` 要求重下（否则"重试 install"永远修不好）。
pub async fn install(state: &UpdaterState, current_version: &Version) -> Result<UpdateView, String> {
    let session = state.session();

    match session.status {
        UpdateStatus::Checking => return Err("检查更新正在进行中，请稍候再试".to_string()),
        UpdateStatus::Downloading => return Err("下载进行中，请先完成下载".to_string()),
        UpdateStatus::Downloaded => {}
        UpdateStatus::Error if matches!(session.error, Some(UpdateError::Install(_))) => {}
        _ => return Err("更新尚未下载完成，请先执行 download".to_string()),
    }
    let artifact = session
        .artifact
        .clone()
        .ok_or_else(|| "更新凭据缺失，请重新 check".to_string())?;
    let path = paths::package_of(&artifact).map_err(|e| format!("{e}，请重新 check"))?;
    let target = session
        .info
        .as_ref()
        .map(|i| i.version.clone())
        .ok_or_else(|| "目标版本缺失，请重新 check".to_string())?;

    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) => {
            return state.fail(UpdateError::Download(format!(
                "下载产物不存在，请重新下载（{}）：{e}",
                path.display()
            )))
        }
    };
    if let Err(e) = verify::verify_artifact(&bytes, &artifact) {
        drop(bytes);
        return state.fail(UpdateError::Download(format!("下载产物校验失败，请重新下载：{e}")));
    }
    drop(bytes);

    // 更新器准备失败属安装阶段问题：重试 install 会重跑 ensure
    let mode = current_mode();
    if let Err(e) = install::ensure_updater(mode).await {
        return state.fail(UpdateError::Install(format!("更新器准备失败：{e}")));
    }
    if let Err(e) = install::launch(mode, &path, current_version, &target) {
        return state.fail(UpdateError::Install(e.to_string()));
    }

    crate::shutdown_hooks::finish_and_exit(0).await
}
