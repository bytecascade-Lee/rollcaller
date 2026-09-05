//! 更新编排服务：把 check / download / install 串成完整流程
//!
//! - [`check`]：读策略 → 调用 updater 检查（命中时携带展示信息与下载凭据）→ 按覆盖
//!   规则把展示信息与凭据一并落 State 会话 → 组装对外结果 [`CheckOutcome`]
//!   （`Available` / `Mandatory` / `NoUpdate` / `Error`）
//! - [`persist_download`]：下载产物（已由 updater::download 完成下载与校验）记入会话
//!   "已下载待安装"
//! - [`install_downloaded`]：读落盘产物 → 验签 → 按运行模式分派安装（NSIS /
//!   Portable），成功路径 `exit(0)` 由安装器 / Go updater 接管
//!
//! State 写入集中在服务层（而非命令层）：无论触发源是按钮还是未来的定时检查，
//! 走本层都会自动维护会话；命令层只做防重入与透传。下载本身在命令层完成
//! （进度事件 Started/Progress/Finished 经 Channel 上报，不入 State；下载与校验在
//! `updater::download` 内完成），本层只负责下载完成后的会话推进。

use crate::common::entity::update::{CheckOutcome, FoundUpdate, Policy};
use crate::common::enums::update::UpdateSource;
use crate::config::app_paths::current_mode;
use crate::service::update::check as updater_check;
use crate::service::update::verify::verify_artifact;
use crate::state::http_client;
use crate::updater::state::{PendingUpdate, UpdaterState};
use anyhow::{anyhow, bail};
use semver::Version;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// 当前用户更新策略
///
/// TODO(设置存储)：策略将来自用户设置（level/channel/allow_downgrade）；当前
/// 尚无设置存储，返回出厂默认（Patch + Stable + 不允许降级）。接入设置后只需
/// 改这一处，check 与 download 复核会自动吃到新策略。
pub fn current_policy() -> Policy {
    Policy::default()
}

/// 检查是否有可用更新，返回对外结果 [`CheckOutcome`]
///
/// 编排职责在本层：取当前版本（`app.package_info().version`）与当前策略，注入运行形态与
/// 缓存目录，调用 `updater::check::check`（返回命中或错误）。随后：
/// - 命中（`Ok(Some(found))`）→ 把展示信息与下载凭据一并按覆盖规则落 State 会话
///   （见 [`update_session`]），再按 `found.force` 组装 `Mandatory`（强制更新，
///   force 只可能是 critical）或 `Available` 返回；
/// - 无更新（`Ok(None)`）→ 清空会话，返回 `NoUpdate`；
/// - 检查失败（`Err`）→ 不触碰会话（保留旧值，前端展示"检查失败"而非"无更新"），
///   统一包装为 `Error`（网络 / 清单不再细分，具体情况在消息中）。
pub async fn check(app: &AppHandle) -> CheckOutcome {
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
    match outcome {
        // 命中目标更新：展示信息与凭据一并落会话；force 决定对外结果是强制还是普通
        Ok(Some(found)) => {
            if let Err(e) = update_session(app, Some(&found), &current_version) {
                tracing::error!("更新会话写入失败：{e}");
            }
            if found.force {
                CheckOutcome::Mandatory(found.info)
            } else {
                CheckOutcome::Available(found.info)
            }
        }
        // 无更新：清空会话（作废旧凭据、清理旧下载产物）
        Ok(None) => {
            if let Err(e) = update_session(app, None, &current_version) {
                tracing::error!("更新会话清理失败：{e}");
            }
            CheckOutcome::NoUpdate
        }
        // 检查失败：不触碰会话，错误原样透出（网络 / 清单统一为 Error）
        Err(e) => CheckOutcome::Error(e.to_string()),
    }
}

/// 按覆盖规则更新 State 会话：同版本重复 check 保留（含已下载产物）；命中其它版本
/// 换新会话；无更新清空会话（均丢弃旧下载产物，尽力清理失败不阻塞）
fn update_session(app: &AppHandle, found: Option<&FoundUpdate>, current_version: &Version) -> anyhow::Result<()> {
    let state = app.state::<UpdaterState>();
    let old = state.pending();

    // 同版本重复 check（如"已下载待安装"时再次检查）：会话原样保留，不丢下载产物
    if let (Some(found), Some(prev)) = (found, &old) {
        if prev.info.version == found.info.version {
            return Ok(());
        }
    }

    // 其余情况：换新会话 / 清空，均丢弃旧的下载产物（尽力清理，失败不阻塞）
    let (next, discarded_path) = match found {
        Some(found) => (
            Some(PendingUpdate::from_found(found, current_version.clone())),
            old.as_ref().and_then(|o| o.downloaded_path.clone()),
        ),
        None => (None, old.as_ref().and_then(|o| o.downloaded_path.clone())),
    };
    state.set_pending(next).map_err(|e| anyhow!("{e}"))?;
    if let Some(path) = discarded_path {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

/// 下载产物落盘后的会话推进：记录"已下载待安装"路径
///
/// 下载与校验（sha256 + 签名）已由 `updater::download` 在落盘前完成，本层只把
/// `downloaded_path` 写入会话并清理上一次的旧下载产物（尽力，失败不阻塞）。
/// install 阶段读回该路径时会再次整体验签（兜底盘上文件损坏）。
pub async fn persist_download(app: &AppHandle, path: PathBuf) -> anyhow::Result<()> {
    let state = app.state::<UpdaterState>();
    let Some(mut session) = state.pending() else {
        bail!("尚未检查更新，请先执行 check_update");
    };

    // 换新产物时清理旧下载产物（尽力清理，失败不阻塞）
    if let Some(old) = &session.downloaded_path {
        if old != &path {
            let _ = std::fs::remove_file(old);
        }
    }
    session.downloaded_path = Some(path);
    state.set_pending(Some(session)).map_err(|e| anyhow!("{e}"))?;
    Ok(())
}

/// 安装已下载的更新：读盘 → 验签 → 启动安装器 → 退出前清理 → `exit(0)`
///
/// 安装的启动分派与退出收尾在 `service/update/install`（[`launch`] / [`finish_and_exit`]）：
/// - Portable：解压 zip → 组装 Go updater config → spawn updater；
/// - Install：ShellExecuteW 启动 NSIS 安装器；
/// - 安装器启动**成功后**才执行退出前清理（如关闭数据库）并退出。
///
/// 任一环节失败返回 `Err`，进程保持存活、资源未清理，用户可重试。
pub async fn install_downloaded(app: &AppHandle) -> anyhow::Result<()> {
    let state = app.state::<UpdaterState>();
    let Some(session) = state.pending() else {
        bail!("尚未下载更新，请先执行 download_update");
    };
    let Some(path) = session.downloaded_path.as_ref() else {
        bail!("更新尚未下载完成，请先执行 download_update");
    };

    // 读盘复验（兜底盘上文件损坏；install 阶段仍整体校验一次）
    let bytes = std::fs::read(path).map_err(|e| anyhow!("读取下载产物失败（{path:?}）：{e}"))?;
    verify_artifact(&bytes, &session.artifact).map_err(|e| anyhow!("下载产物校验失败：{e}"))?;
    drop(bytes);

    // 启动安装器（成功即接管；失败则进程存活、可重试，不触发退出清理）
    crate::service::update::install::launch(
        current_mode(),
        path,
        &session.current_version,
        &session.info.version,
    )?;

    // 安装器已接管：统一收尾（执行退出前清理并 exit(0)，永不返回）
    crate::service::update::install::finish_and_exit().await
}
