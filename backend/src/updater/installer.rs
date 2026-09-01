//! 安装分派：识别载荷类型并交给对应的安装器执行
//!
//! - 任务 05：NSIS 安装包（`installer/nsis.rs`）
//! - 任务 06：便携版 zip + Go updater 编排（`installer/portable.rs`）

pub mod nsis;
pub mod portable;

use nsis::{install_nsis, NsisOptions};
use portable::{install_portable, PortableOptions};

/// 已识别出的载荷安装器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallKind {
    /// NSIS 安装包（.exe 或 zip 内嵌 .exe）
    Nsis,
    /// 便携版 zip（内含整个应用目录，由独立 Go updater 接管）
    Portable,
}

/// 与 `InstallKind` 配套的安装选项（按值传递：内含 `FnOnce` 退出钩子，只能执行一次；
/// 成功启动安装器后本进程即 `exit(0)`，调用方不会再使用）
pub enum InstallOptions {
    Nsis(NsisOptions),
    Portable(PortableOptions),
}

/// 按载荷类型分派安装
#[cfg(windows)]
pub fn install(kind: InstallKind, bytes: &[u8], opts: InstallOptions) -> anyhow::Result<()> {
    match (kind, opts) {
        (InstallKind::Nsis, InstallOptions::Nsis(opts)) => install_nsis(bytes, opts),
        (InstallKind::Portable, InstallOptions::Portable(opts)) => install_portable(bytes, opts),
        (kind, _) => anyhow::bail!("安装类型与选项不匹配: {kind:?}"),
    }
}

/// 非 Windows 平台：安装不可用（桩）
#[cfg(not(windows))]
pub fn install(_kind: InstallKind, _bytes: &[u8], _opts: InstallOptions) -> anyhow::Result<()> {
    anyhow::bail!("安装仅支持 Windows")
}
