//! 更新编排服务：把 check / download / verify / install 串成完整流程
//!
//! - [`check`]：读端点 → 拉取清单 → 版本判定 → 返回更新信息
//! - [`download_and_install`]：验签下载字节 → 按运行模式分派安装（NSIS / Portable）
//!
//! 下载本身在命令层（`updater::commands`）完成并上报进度事件（Started /
//! Progress / Finished），本服务只做"验签 + 安装分派"——职责按任务 02/03/05/06
//! 划分，编排层只串流程。安装成功路径由安装器 / Go updater 接管（`exit(0)` 不返回）。

use crate::config::app_paths::{current_mode, AppMode};
use crate::state::http_client;
use crate::updater::check::{self as updater_check, UpdateInfo, UpdateKind};
use crate::updater::download::DownloadProgress;
use crate::updater::installer::nsis::NsisOptions;
use crate::updater::installer::portable::PortableOptions;
use crate::updater::installer::{install, InstallKind, InstallOptions};
use crate::updater::verify;
use serde::Serialize;
use tauri::AppHandle;

/// 检查结果包装（命令层返回给前端，`update` 为 `None` 时对应前端契约的 `null`）
#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    pub update: Option<UpdateInfo>,
}

/// 检查是否有可用更新
///
/// 当前版本取自 `app.package_info().version`，不允许降级。
pub async fn check(app: &AppHandle) -> anyhow::Result<Option<UpdateInfo>> {
    updater_check::check(
        http_client::get_client(),
        updater_check::CHECK_ENDPOINT,
        &app.package_info().version,
        false, // 不允许降级
    )
    .await
}

/// 验签并安装已下载的更新字节（按运行模式分派）
///
/// `bytes` 已由调用方（命令层）通过 `updater::download` 下载，本函数先验签、
/// 再安装；验签失败立即返回错误，不进入安装（不落盘）。
///
/// `_app` / `_on_progress` 参数为兼容规格签名保留（当前实现不需要窗口/包信息，
/// 下载已在调用方完成、安装阶段无进度上报），后续接入安装选项来源时使用。
///
/// 安装器由 `info.kind` 决定：Portable 模式可能选中 nsis 载荷（见
/// `check.rs::select_artifact` 的次选规则），此时仍应走 NSIS 安装器；
/// 选项按 `current_mode()` 构造（Develop 模式不更新）。
///
/// 两条安装路径成功后都会 `exit(0)`（安装器 / Go updater 接管），
/// 本函数正常路径不会返回；失败返回 `Err`，进程保持存活以便上层提示用户。
pub async fn download_and_install(
    _app: &AppHandle,
    info: &UpdateInfo,
    bytes: Vec<u8>,
    _on_progress: impl FnMut(DownloadProgress),
) -> anyhow::Result<()> {
    // 1. 验签：失败立即返回错误，不进入安装（不落盘）
    verify::verify_artifact(&bytes, &info.artifact)?;

    // 2. 安装器由 info.kind 决定（见函数文档：Portable 模式可能选中 nsis 载荷）
    let kind = match info.kind {
        UpdateKind::Nsis => InstallKind::Nsis,
        UpdateKind::Portable => InstallKind::Portable,
    };

    // 3. 选项按运行模式构造（Develop 不更新）
    let opts = match current_mode() {
        AppMode::Develop => anyhow::bail!("开发模式不更新"),
        AppMode::Install => InstallOptions::Nsis(NsisOptions::default()),
        AppMode::Portable => InstallOptions::Portable(portable_options(info)),
    };

    install(kind, &bytes, opts)
}

/// 由当前环境推导便携版安装选项
///
/// - `target_dir`：便携版 = exe 所在目录（`app_paths::root_dir()`）
/// - `preserve_paths`：用户数据目录（更新时不随版本清空）
/// - `launch_args`：原启动参数（透传给新进程，与 NSIS 的 `/ARGS` 语义一致）
/// - `exe_name`：当前可执行文件名（`std::env::current_exe`）
fn portable_options(info: &UpdateInfo) -> PortableOptions {
    PortableOptions {
        version: info.version.to_string(),
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
