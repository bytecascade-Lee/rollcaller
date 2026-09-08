//! 更新编排服务：领域子模块聚合 + check / download / cancel / install 编排
//!
//! # 分层
//!
//! - 领域子模块（`version` / `check` / `verify` / `download` / `install`）：纯逻辑，
//!   不感知状态与 Tauri；
//! - 本文件编排函数：纯 Rust 编排——不依赖 `AppHandle`，全部输入显式传参
//!   （`&UpdaterState` + 所需依赖）；对外统一返回裁剪的展示视图 [`UpdateView`]。
//!   事件分两路、由命令层注入的回调发出：**状态迁移帧**（进入 Downloading 等，view
//!   通道）与**进度窄帧**（download 通道，节流后）各自独立；命令返回的终态视图由
//!   命令层再广播。进度数据落 state 原子槽（[`crate::state::update::DownloadSlot`]），
//!   下载过程不碰 session 锁。
//!
//! # 状态机约定（编排层负责推进）
//!
//! | 阶段 | 可进入的命令 | 下一阶段 |
//! |---|---|---|
//! | Idle | check | Checking → UpToDate / Available / Downloaded / Error |
//! | Checking | —（防重入） | 同上 |
//! | UpToDate / Available / Downloaded / Error | check | 同上 |
//! | Available / Error(Download) | download | Downloading |
//! | Downloading | cancel（置取消位） | Available（取消）/ Downloaded（成功）/ Error(Download)（失败） |
//! | Downloaded | cancel / install | Available（取消删产物）/ exit(0)（成功）/ Error(Install)（启动失败）/ Error(Download)（产物缺失损坏） |
//!
//! 错误收敛为 `status = Error` + `error: Option<UpdateError>`（Check / Download /
//! Install 带载荷），视图以 `Error(UpdateError)` 表达，前端据其 `type` 给"重试"
//! 按钮（重试命令 == 失败阶段）；下载取消回 `Available`；失败保留凭据（`artifact`）
//! 可重试——**产物路径不入会话**，任何时刻由
//! `artifact.file_name()` + [`paths::package`] 从凭据现推并配合磁盘探测，磁盘才是产物事实源。
//!
//! # 产物与就绪
//!
//! 产物目录布局统一由 [`paths`] 管理：下载产物平铺于 `temp/update/packages/`、
//! 文件名取发布产物 url 的最后一段（`Artifact::file_name()`，自带版本等完整信息），
//! 下载中数据暂存 `temp/downloads/` 的随机 `.part`。
//! "已下载待安装"以磁盘事实表达：目标文件存在且校验通过（[`ready_artifact`]）
//! 即为就绪；check 命中与 download 入口都会先探测，命中则跳过下载（不跳过验签）。

mod check;
mod download;
mod install;
mod paths;
mod verify;
mod version;

use crate::common::entity::update::{Artifact, DownloadProgress, Policy};
use crate::common::enums::update::{UpdateDecision, UpdateError, UpdateSource, UpdateStatus, UpdateView};
use crate::config::app_paths::{current_mode, AppMode};
use crate::state::http_client;
use crate::state::update::UpdaterState;
use anyhow::anyhow;
use semver::Version;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// 下载进度广播的最小间隔（进度过高频时会节流）
const PROGRESS_BROADCAST_INTERVAL: Duration = Duration::from_millis(100);

/// 当前用户更新策略
///
/// TODO(设置存储)：策略将来自用户设置（level/channel）；当前尚无设置存储，
/// 返回出厂默认（Patch + Stable）。接入设置后只需改这一处。
pub fn current_policy() -> Policy {
    Policy::default()
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

/// 检查是否有可用更新（编排）
///
/// 阶段守卫：`Checking` / `Downloading` 拒绝重入。结果一律以视图返回：
/// 命中 → 目标产物已在磁盘且校验通过则 `Downloaded`（恢复现场），否则
/// `Available`（severity=critical 即强制更新）；无更新 → `UpToDate`；
/// 失败 → `Error(Check)`（保留原会话内容，供重试后覆盖）。
pub async fn check(state: &UpdaterState, current_version: &Version) -> anyhow::Result<UpdateView> {
    // 入口守卫并置 Checking（此刻起防重入）
    state
        .mutate(|s| {
            if matches!(s.status, UpdateStatus::Checking | UpdateStatus::Downloading) {
                return Err("更新操作正在进行中，请稍候再试".to_string());
            }
            s.status = UpdateStatus::Checking;
            s.error = None;
            Ok(())
        })
        .map_err(|e| anyhow!(e.to_string()))?;

    let policy = current_policy();
    let mode = current_mode();
    let source = match mode {
        AppMode::Develop => UpdateSource::Develop,
        AppMode::Install | AppMode::Portable => UpdateSource::CNB,
    };
    let outcome = check::check(http_client::client(), source, current_version, &policy, mode).await;

    // 锁外磁盘探测：以"产物文件存在且校验通过"为已下载判据——磁盘为唯一事实源，
    // 会话不存产物路径；命中且就绪 → 恢复为 Downloaded（可跳过下载直接安装），
    // 文件损坏则清除待重下。检查失败不触碰磁盘。
    let ready_path = match &outcome {
        Ok(Some(found)) => found
            .artifact
            .file_name()
            .map(|name| paths::package(&name))
            .and_then(|path| ready_artifact(&path, &found.artifact)),
        _ => None,
    };

    let result = state
        .mutate(|s| {
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
                    s.error = None;
                }
                // 无更新：清空会话（作废旧凭据）
                Ok(None) => {
                    s.status = UpdateStatus::UpToDate;
                    s.info = None;
                    s.severity = Default::default();
                    s.artifact = None;
                    s.error = None;
                }
                // 检查失败：保留原会话内容，仅落错误（重试 check 后覆盖）
                Err(e) => {
                    s.status = UpdateStatus::Error;
                    s.error = Some(UpdateError::Check(e.to_string()));
                }
            }
            Ok(())
        })
        .map_err(|e| e.to_string());

    Ok(result.unwrap())
}

/// 下载已批准产物（编排）
///
/// 阶段守卫：`Downloaded` 幂等直接返回；`Checking` / `Downloading` 拒绝；
/// 仅 `Available` 或携带下载凭据的 `Error(Download)` 允许进入。
/// 产物落点由 `paths` 统一管理（packages 平铺）；下载前先探测就绪锚点——目标文件
/// 已存在则不重复下载，但仍整体验签（校验失败清除重下）。
///
/// 对外信号分两路（均无 Tauri 依赖，由命令层注入闭包发出）：
/// - `on_view`：状态迁移帧——确认进入下载时广播一次 `Downloading`（view 通道）；
/// - `on_progress`：进度窄帧——每 chunk 只写 state 原子槽（不碰 session 锁），
///   节流达 [`PROGRESS_BROADCAST_INTERVAL`] 后从槽读快照回调（download 通道）。
/// 进度是瞬时数据、只经窄帧广播，不落 session；终态视图由命令层在返回后统一广播。
///
/// `current_version` 为复核基线（与 check 同源，由命令层从 `package_info` 传入），
/// 不再存入会话。
pub async fn download(
    state: &UpdaterState,
    current_version: &Version,
    on_view: impl Fn(&UpdateView),
    on_progress: impl Fn(&DownloadProgress),
) -> Result<UpdateView, String> {
    // 幂等：已下载完成 → 直接返回（前端可进入"已就绪"）
    if state.session().status == UpdateStatus::Downloaded {
        return Ok(state.view());
    }

    // 入口守卫并置 Downloading
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
    // 入口复核：策略变更 → 旧凭据作废，落 Error(Check) 要求重查（基线版本为命令层传入，
    // 与 check 同源；会话不再保存 current_version）
    let decision = match session.info.as_ref().map(|i| &i.version) {
        Some(target) => check::evaluate(current_version, target, &current_policy(), session.severity),
        None => return Err("更新凭据不完整，请重新执行 check".to_string()),
    };
    if decision != UpdateDecision::Update {
        state.mutate(|s| {
            s.status = UpdateStatus::Error;
            s.error = Some(UpdateError::Check("更新设置已变更，请重新执行 check".to_string()));
            Ok(())
        })?;
        return Ok(state.view());
    }
    let artifact = session
        .artifact
        .clone()
        .ok_or_else(|| "更新凭据不完整，请重新执行 check".to_string())?;
    // 最终产物路径 = packages/ + url 最后一段（布局收口于 paths）；url 无文件名视为清单不合法
    let final_path = paths::package(
        &artifact
            .file_name()
            .ok_or_else(|| "下载地址缺少文件名，请重新执行 check".to_string())?,
    );

    // 就绪锚点：目标文件已存在 → 跳过下载，但不能跳过校验；损坏清除重下
    if ready_artifact(&final_path, &artifact).is_some() {
        return state
            .mutate(|s| {
                s.status = UpdateStatus::Downloaded;
                s.error = None;
                Ok(())
            })
            .map_err(|e| e.to_string());
    }

    // 确认真正进入下载：先广播迁移帧（Available → Downloading，view 通道），
    // 再流式下载（part 工作区 → 校验 → 重命名 packages）。
    // 进度每 chunk 只写 state 原子槽（不碰 session 锁），节流达阈值后从槽读快照
    // 回调 on_progress（download 通道窄帧）；进度是瞬时广播、不落 session。
    on_view(&state.view());
    let mut last_broadcast = Instant::now();
    let result = download::download(&artifact, state.is_cancelled(), |p: DownloadProgress| {
        state.slot().set_progress(p);
        if last_broadcast.elapsed() >= PROGRESS_BROADCAST_INTERVAL {
            on_progress(&state.slot().snapshot());
            last_broadcast = Instant::now();
        }
    })
        .await;
    // 无论成功 / 失败 / 取消都复位取消标志与进度槽
    state.reset_cancel();
    state.slot().reset();

    match result {
        Ok(_path) => state
            .mutate(|s| {
                s.status = UpdateStatus::Downloaded;
                s.error = None;
                Ok(())
            })
            .map_err(|e| e.to_string()),
        Err(e) => {
            let msg = e.to_string();
            // 用户取消 → 回 Available（可重下），不视为错误
            if msg.contains("CANCELLED") {
                state
                    .mutate(|s| {
                        s.status = UpdateStatus::Available;
                        s.error = None;
                        Ok(())
                    })
                    .map_err(|e| e.to_string())
            } else {
                state
                    .mutate(|s| {
                        s.status = UpdateStatus::Error;
                        s.error = Some(UpdateError::Download(msg));
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
/// - 已下载完成（取消晚于完成）：按凭据现推产物路径删除，会话回 `Available`
///   （保留 info/凭据，可重下；产物不存在也无妨，磁盘为事实源）。
pub fn cancel(state: &UpdaterState) -> Result<UpdateView, String> {
    let session = state.session();

    // 已下载完成：删产物回可重下态（产物路径 = packages/ + 凭据文件名，现推）
    if session.status == UpdateStatus::Downloaded {
        if let Some(artifact) = session.artifact.as_ref() {
            if let Some(name) = artifact.file_name() {
                let _ = std::fs::remove_file(paths::package(&name));
            }
        }
        return state.mutate(|s| {
            s.status = UpdateStatus::Available;
            s.error = None;
            Ok(())
        });
    }
    // 下载中：置取消位，由下载流程收尾回 Available
    if session.status == UpdateStatus::Downloading {
        state.request_cancel();
    }
    Ok(state.view())
}

/// 安装已下载产物（编排；成功路径进程退出，安装器接管）
///
/// 阶段守卫：仅 `Downloaded` 或 `Error(Install)`（重试）允许进入。
/// 产物路径由凭据现推（会话不存路径）：读盘 → 整体验签（对完整安装包二次校验）
/// → Portable 先 ensure 更新器（幂等下载 / 校验，见 [`install::ensure_updater`]）
/// → 启动安装器（成功即退出）。**产物缺失或校验失败 = 磁盘事实被推翻**，落
/// `Error(Download)` 让用户重新下载（而非 Error(Install) 死循环）；仅更新器准备
/// 失败 / 安装器启动失败保留 `Error(Install)` 可重试。
///
/// `current_version` 为安装器入参（from），由命令层从 `package_info` 传入。
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
    // 产物路径 = packages/ + 凭据文件名，现推（会话不存路径）
    let path = paths::package(
        &artifact
            .file_name()
            .ok_or_else(|| "下载地址缺少文件名，请重新 check".to_string())?,
    );
    let target = session
        .info
        .as_ref()
        .map(|i| i.version.clone())
        .ok_or_else(|| "目标版本缺失，请重新 check".to_string())?;

    // 读盘复验（install 阶段对完整安装包整体验签，兜底校验损坏；磁盘为事实源——
    // Downloaded 状态不代表产物仍有效，缺失 / 损坏一律请重下）
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            state.mutate(|s| {
                s.status = UpdateStatus::Error;
                s.error = Some(UpdateError::Download(format!(
                    "下载产物不存在，请重新下载（{}）：{e}",
                    path.display()
                )));
                Ok(())
            })?;
            return Ok(state.view());
        }
    };
    if let Err(e) = verify::verify_artifact(&bytes, &artifact) {
        drop(bytes);
        state.mutate(|s| {
            s.status = UpdateStatus::Error;
            s.error = Some(UpdateError::Download(format!("下载产物校验失败，请重新下载：{e}")));
            Ok(())
        })?;
        return Ok(state.view());
    }
    drop(bytes);

    // Portable：先确保更新器就绪（幂等下载 / 校验到 cache/update/bin），再启动安装。
    // 更新器准备失败属安装阶段问题，落 Error(Install)——重试 install 会重跑 ensure。
    let mode = current_mode();
    if let Err(e) = install::ensure_updater(mode).await {
        state.mutate(|s| {
            s.status = UpdateStatus::Error;
            s.error = Some(UpdateError::Install(format!("更新器准备失败：{e}")));
            Ok(())
        })?;
        return Ok(state.view());
    }

    // 启动安装器（成功即接管；失败则进程存活、产物保留可重试）
    if let Err(e) = install::launch(mode, &path, current_version, &target) {
        state.mutate(|s| {
            s.status = UpdateStatus::Error;
            s.error = Some(UpdateError::Install(e.to_string()));
            Ok(())
        })?;
        return Ok(state.view());
    }

    // 安装器已接管：统一收尾（执行退出前清理并 exit(0)，永不返回）
    install::finish_and_exit().await
}
