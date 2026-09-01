//! 安装分派：识别载荷类型并交给对应的安装器执行
//!
//! - 任务 05：NSIS 安装包（本文件 + `installer/nsis.rs`）
//! - 后续任务：便携版（交由独立 Go 更新器执行）

pub mod nsis;

use nsis::{install_nsis, NsisOptions};

/// 已识别出的载荷安装器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallKind {
    /// NSIS 安装包（.exe 或 zip 内嵌 .exe）
    Nsis,
}

/// 按载荷类型分派安装；当前只有 NSIS 一种
///
/// `opts` 按值传递：`NsisOptions` 内含 `FnOnce` 退出钩子（只能执行一次），
/// 且 NSIS 成功启动后本进程即 `exit(0)`，调用方不会再使用 opts。
#[cfg(windows)]
pub fn install(kind: InstallKind, bytes: &[u8], opts: NsisOptions) -> anyhow::Result<()> {
    match kind {
        InstallKind::Nsis => install_nsis(bytes, opts),
    }
}

/// 非 Windows 平台：NSIS 安装不可用（桩）
#[cfg(not(windows))]
pub fn install(_kind: InstallKind, _bytes: &[u8], _opts: NsisOptions) -> anyhow::Result<()> {
    anyhow::bail!("NSIS 安装仅支持 Windows")
}
