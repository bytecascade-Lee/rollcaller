//! 更新编排服务：把 check / download / install 串成完整流程
//!
//! - [`check`]：读策略 → 拉取清单 → severity/force 融合判定 → 按覆盖规则落
//!   State 会话 → 返回更新信息
//! - [`persist_download`]：对下载字节验签 → 落盘 → 会话标记"已下载待安装"
//! - [`install_downloaded`]：读落盘产物 → 验签 → 按运行模式分派安装（NSIS /
//!   Portable），成功路径 `exit(0)` 由安装器 / Go updater 接管
//!
//! State 写入集中在服务层（而非命令层）：无论触发源是按钮还是未来的定时检查，
//! 走本层都会自动维护会话；命令层只做防重入与透传。下载本身在命令层完成
//! （进度事件 Started/Progress/Finished 经 Channel 上报，不入 State），本层
//! 只负责下载完成后的"验签 + 落盘 + 会话推进"。

use crate::config::app_paths::{current_mode, AppMode};
use crate::state::http_client;
use crate::updater::check::{self as updater_check, UpdateInfo, UpdateKind};
use crate::updater::installer::nsis::NsisOptions;
use crate::updater::installer::portable::PortableOptions;
use crate::updater::installer::{install, InstallKind, InstallOptions};
use crate::updater::state::{PendingUpdate, UpdaterState};
use crate::updater::verify::verify_artifact;
use crate::updater::version_policy::Policy;
use anyhow::{anyhow, bail};
use semver::Version;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// 检查结果包装（命令层返回给前端，`update` 为 `None` 时对应前端契约的 `null`）
#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    pub update: Option<UpdateInfo>,
}

/// 当前用户更新策略
///
/// TODO(设置存储)：策略将来自用户设置（level/channel/allow_downgrade）；当前
/// 尚无设置存储，返回出厂默认（Patch + Stable + 不允许降级）。接入设置后只需
/// 改这一处，check 与 download 复核会自动吃到新策略。
pub fn current_policy() -> Policy {
    Policy::default()
}

/// 检查是否有可用更新（含 severity 豁免与 force 合法性判定）
///
/// 当前版本取自 `app.package_info().version`；判定通过后按覆盖规则落 State 会话：
///
/// | 场景 | 动作 |
/// |---|---|
/// | 有更新且版本 ≠ 现会话 | 换新会话（清理旧下载产物） |
/// | 有更新且版本 == 现会话 | 保留现会话（含已下载产物，避免重复下载） |
/// | 无更新 | 清空会话（作废旧凭据、清理旧下载产物） |
///
/// 返回给前端展示的更新信息；网络/清单失败返回 `Err`（不触碰 State，保留旧值）。
pub async fn check(app: &AppHandle) -> anyhow::Result<Option<UpdateInfo>> {
    let policy = current_policy();
    let current_version = app.package_info().version.clone();
    let info = updater_check::check(
        http_client::get_client(),
        updater_check::CNB,
        &current_version,
        &policy,
    )
    .await?;
    update_session(app, info.as_ref(), &current_version)?;
    Ok(info)
}

/// 按覆盖规则更新 State 会话（见 [`check`] 文档的表格）
fn update_session(
    app: &AppHandle,
    update: Option<&UpdateInfo>,
    current_version: &Version,
) -> anyhow::Result<()> {
    let state = app.state::<UpdaterState>();
    let old = state.pending();

    // 同版本重复 check（如"已下载待安装"时再次检查）：会话原样保留，不丢下载产物
    if let (Some(info), Some(prev)) = (update, &old) {
        if prev.version == info.version {
            return Ok(());
        }
    }

    // 其余情况：换新会话 / 清空，均丢弃旧的下载产物（尽力清理，失败不阻塞）
    let (next, discarded_path) = match update {
        Some(info) => (
            Some(PendingUpdate::from_update(info, current_version.clone())),
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

/// 下载完成后的收尾：验签 → 落盘 → 会话标记"已下载待安装"
///
/// 验签失败立即报错（不落盘，下载阶段暴露坏包）；落盘目录为
/// `temp_dir()/updater_downloads`，文件名为 `rollcaller-{version}-{kind}.bin`，
/// install 阶段读回并再次整体验签（兜底盘上文件损坏）。
pub async fn persist_download(app: &AppHandle, bytes: Vec<u8>) -> anyhow::Result<()> {
    let state = app.state::<UpdaterState>();
    let Some(mut session) = state.pending() else {
        bail!("尚未检查更新，请先执行 check_update");
    };

    verify_artifact(&bytes, &session.artifact).map_err(|e| anyhow!("下载内容校验失败：{e}"))?;

    let path = download_path_for(&session)?;
    std::fs::write(&path, &bytes).map_err(|e| anyhow!("写入下载产物失败：{e}"))?;
    session.downloaded_path = Some(path);
    state.set_pending(Some(session)).map_err(|e| anyhow!("{e}"))?;
    Ok(())
}

/// 安装已下载的更新：读盘 → 验签 → 按运行模式分派
///
/// 成功路径由安装器 / Go updater 接管（`exit(0)`，正常不返回）；失败返回 `Err`，
/// 进程保持存活以便上层提示，用户可重试安装。开发模式不更新（防御）。
pub async fn install_downloaded(app: &AppHandle) -> anyhow::Result<()> {
    let state = app.state::<UpdaterState>();
    let Some(session) = state.pending() else {
        bail!("尚未下载更新，请先执行 download_update");
    };
    let Some(path) = session.downloaded_path.as_ref() else {
        bail!("更新尚未下载完成，请先执行 download_update");
    };

    let bytes =
        std::fs::read(path).map_err(|e| anyhow!("读取下载产物失败（{path:?}）：{e}"))?;
    verify_artifact(&bytes, &session.artifact).map_err(|e| anyhow!("下载产物校验失败：{e}"))?;

    // 安装器由会话的 kind 决定（Portable 模式可能选中 nsis 载荷，此时仍走 NSIS）
    let kind = match session.kind {
        UpdateKind::Nsis => InstallKind::Nsis,
        UpdateKind::Portable => InstallKind::Portable,
    };
    // 选项按运行模式构造（Develop 不更新）
    let opts = match current_mode() {
        AppMode::Develop => bail!("开发模式不更新"),
        AppMode::Install => InstallOptions::Nsis(NsisOptions::default()),
        AppMode::Portable => InstallOptions::Portable(portable_options(&session.version.to_string())),
    };

    install(kind, &bytes, opts)
}

/// 下载产物路径：`temp_dir()/updater_downloads/rollcaller-{version}-{kind}.bin`
fn download_path_for(session: &PendingUpdate) -> anyhow::Result<PathBuf> {
    let kind = match session.kind {
        UpdateKind::Nsis => "nsis",
        UpdateKind::Portable => "portable",
    };
    let dir = crate::config::app_paths::temp_dir().join("updater_downloads");
    std::fs::create_dir_all(&dir).map_err(|e| anyhow!("创建下载目录失败：{e}"))?;
    Ok(dir.join(format!("rollcaller-{}-{kind}.bin", session.version)))
}

/// 由版本推导便携版安装选项
///
/// - `target_dir`：便携版 = exe 所在目录（`app_paths::root_dir()`）
/// - `preserve_paths`：用户数据目录（更新时不随版本清空）
/// - `launch_args`：原启动参数（透传给新进程，与 NSIS 的 `/ARGS` 语义一致）
/// - `exe_name`：当前可执行文件名（`std::env::current_exe`）
fn portable_options(version: &str) -> PortableOptions {
    PortableOptions {
        version: version.to_string(),
        target_dir: crate::config::app_paths::root_dir().to_path_buf(),
        preserve_paths: vec![crate::config::app_paths::data_dir().to_path_buf()],
        launch_args: std::env::args_os().skip(1).collect(),
        exe_name: std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|f| f.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "rollcaller.exe".to_string()),
        on_before_exit: None,
    }
}
