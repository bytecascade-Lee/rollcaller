//! NSIS 安装器执行
//!
//! # 流程：
//! 1. 构造安装器命令行；
//! 2. `ShellExecuteW` 启动安装器；
//! 3. 成功后返回 `Ok(())`——由调用方执行退出前清理
//!    （`shutdown_hooks::run_all()`，如关闭数据库）后 `exit(0)`，安装器负责重启应用。
//!
//! 各函数注释中标注了源行号，便于日后对照升级。转义函数不得自创/简化。

use anyhow::anyhow;
use std::ffi::{OsStr, OsString};
use std::io::Cursor;
use std::path::PathBuf;

/// NSIS 安装界面模式（对应插件 `WindowsUpdateInstallMode` 的 NSIS 分支，config.rs:14-23）
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NsisInstallMode {
    /// 有基础 UI（含最终对话框），不附加界面参数
    BasicUi,
    /// 完全静默，无任何窗口
    Silent,
    /// 无人值守，仅显示进度条（默认）
    #[default]
    Passive,
}
#[cfg(target_os = "windows")]
impl NsisInstallMode {
    /// 界面模式对应的 NSIS 参数（与插件 config.rs:41-50 一致）
    pub fn nsis_args(&self) -> &'static [&'static str] {
        match self {
            /// `Passive` → `/P`
            Self::Passive => &["/P"],
            /// `Silent`  → `/S`
            Self::Silent => &["/S"],
            /// `BasicUi` → 无
            Self::BasicUi => &[],
        }
    }

    /// 安装完成后要求重启时附加的 NSIS 参数（与插件 config.rs:53-58 一致）
    ///
    /// - `BasicUi` → 无（自带 UI，不自动重启）
    /// - 其余     → `/R`
    pub fn nsis_restart_after_install_args(&self) -> &'static [&'static str] {
        match self {
            Self::BasicUi => &[],
            _ => &["/R"],
        }
    }
}

/// NSIS 安装选项（规格 2.4）
#[cfg(target_os = "windows")]
pub struct NsisOptions {
    /// 安装界面模式（默认 Passive）
    pub mode: NsisInstallMode,
    /// 是否在安装完成后重启应用并继承原启动参数（默认 true）
    pub restart_after_install: bool,
    /// 用户自定义安装器参数（追加在命令行末尾；来源待配置化，本任务先留空）
    pub args: Vec<OsString>,
}
#[cfg(target_os = "windows")]
impl Default for NsisOptions {
    fn default() -> Self {
        Self {
            mode: NsisInstallMode::Passive,
            restart_after_install: true,
            args: Vec::new(),
        }
    }
}

/// 安装入口：构造参数 → `ShellExecuteW` 启动安装器（不 `exit`）
///
/// 对应插件 `install_inner`（updater.rs:835-877）。`exec_path` 应为已下载并校验
/// （sha256 + 签名）的安装包路径，本函数不再校验。
///
/// 失败（如 `ShellExecuteW` 返回值 <= 32）返回 `Err`，进程保持存活交由上层提示；
/// 成功后安装器接管（自行重启应用），**本进程应随即退出**：调用方需先执行
/// `shutdown_hooks::run_all().await`（关闭数据库等）再 `std::process::exit(0)`。
#[cfg(target_os = "windows")]
pub fn install(exec_path: PathBuf, opts: NsisOptions) -> anyhow::Result<()> {
    if !exec_path.is_file() {
        return Err(anyhow!("安装器路径不存在或不是文件: {:?}", exec_path));
    }

    // 1. 构造安装器命令行
    let args = build_nsis_args(&opts);
    tracing::debug!("Executing updater {:?} with parameters: {:?}", &exec_path, args);

    // 2. ShellExecuteW 启动，成功后安装器接管（进程退出由调用方完成）
    let file = encode_wide(&exec_path);
    let parameters = encode_wide(&args);

    use windows_sys::{
        w,
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOW},
    };

    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            w!("open"),
            file.as_ptr(),
            parameters.as_ptr(),
            std::ptr::null(),
            SW_SHOW,
        )
    };
    check_shell_execute_result(result as isize)
}

#[cfg(not(target_os = "windows"))]
pub fn install(_exec_path: PathBuf, _opts: NsisOptions) -> anyhow::Result<()> {
    anyhow::bail!("NSIS 安装仅支持 Windows")
}

/// 构造传给 NSIS 安装器的完整命令行参数
///
/// 对应插件 `updater_parameters` 的 NSIS 分支（updater.rs:879-909）：
/// `<installMode 参数> /UPDATE [/R /ARGS <转义后的原启动参数...>] [用户自定义 installer_args]`
#[cfg(target_os = "windows")]
pub(crate) fn build_nsis_args(opts: &NsisOptions) -> OsString {
    let current_exe_args: Vec<OsString> = std::env::args_os().skip(1).collect();
    compose_nsis_args(opts.mode, opts.restart_after_install, &current_exe_args, &opts.args)
}

/// 参数拼装的纯函数（便于单测）：不读进程环境，所有输入显式传入
#[cfg(target_os = "windows")]
fn compose_nsis_args(
    install_mode: NsisInstallMode,
    restart_after_install: bool,
    current_exe_args: &[OsString],
    installer_args: &[OsString],
) -> OsString {
    let mut parts: Vec<&OsStr> = Vec::new();
    parts.extend(install_mode.nsis_args().iter().map(OsStr::new));
    parts.push(OsStr::new("/UPDATE"));

    // 与插件一致：`escaped` 声明在 if 之外，使其存活到 join 之后（updater.rs:889-904）
    let escaped: Vec<OsString>;
    if restart_after_install {
        escaped = current_exe_args.iter().map(escape_nsis_current_exe_arg).collect();
        parts.extend(install_mode.nsis_restart_after_install_args().iter().map(OsStr::new));
        parts.push(OsStr::new("/ARGS"));
        parts.extend(escaped.iter().map(OsString::as_os_str));
    }

    parts.extend(installer_args.iter().map(OsString::as_os_str));
    parts.join(OsStr::new(" "))
}

// ---------------------------------------------------------------------------
// 以下两个函数原样搬运自 tauri-plugin-updater v2（plugins-workspace v2 分支）：
//   `escape_nsis_current_exe_arg` —— updater.rs:1570-1609
//   `encode_wide`                 —— updater.rs:1544-1553
// 转义规则不得自创/简化，保留插件实现与全部测试用例，仅注释补充来源行号。
// ---------------------------------------------------------------------------

/// 转义单个原启动参数，避免 NSIS 把 `/` 开头的内容当作安装器自身参数
///
/// adapted from
/// https://github.com/rust-lang/rust/blob/1c047506f94cd2d05228eb992b0a6bbed1942349/library/std/src/sys/args/windows.rs#L174
///（来源：插件 updater.rs:1570）
#[cfg(target_os = "windows")]
fn escape_nsis_current_exe_arg(arg: impl AsRef<OsStr>) -> OsString {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    let arg = arg.as_ref();
    let mut cmd: Vec<u16> = Vec::new();

    // compared to std we additionally escape `/` so that nsis won't interpret them as a beginning of an nsis argument.
    let quote = arg.as_encoded_bytes().iter().any(|c| *c == b' ' || *c == b'\t' || *c == b'/') || arg.is_empty();
    let escape = true;
    if quote {
        cmd.push('"' as u16);
    }
    let mut backslashes: usize = 0;
    for x in arg.encode_wide() {
        if escape {
            if x == '\\' as u16 {
                backslashes += 1;
            } else {
                if x == '"' as u16 {
                    // Add n+1 backslashes to total 2n+1 before internal '"'.
                    cmd.extend((0..=backslashes).map(|_| '\\' as u16));
                }
                backslashes = 0;
            }
        }
        cmd.push(x);
    }
    if quote {
        // Add n backslashes to total 2n before ending '"'.
        cmd.extend((0..backslashes).map(|_| '\\' as u16));
        cmd.push('"' as u16);
    }
    OsString::from_wide(&cmd)
}

/// 将路径/参数转为 UTF-16（含结尾 `\0`），供 `ShellExecuteW` 使用（来源：插件 updater.rs:1544）
#[cfg(target_os = "windows")]
fn encode_wide(string: impl AsRef<OsStr>) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    string.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

/// 校验 `ShellExecuteW` 返回值：> 32 表示成功，其余为错误（与插件 install_inner 判定一致，
/// updater.rs:872-874）。抽出为独立函数便于单测该分支。
#[cfg(target_os = "windows")]
fn check_shell_execute_result(result: isize) -> anyhow::Result<()> {
    if result <= 32 {
        return Err(anyhow::anyhow!(
            "ShellExecuteW 启动安装器失败: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn it_escapes_correctly_for_nsis() {
        let cases = [
            "something",
            "--flag",
            "--empty=",
            "--arg=value",
            "some space",                     // This simulates `./my-app "some string"`.
            "--arg value", // -> This simulates `./my-app "--arg value"`. Same as above but it triggers the startsWith(`-`) logic.
            "--arg=unwrapped space", // `./my-app --arg="unwrapped space"`
            "--arg=\"wrapped\"", // `./my-app --args=""wrapped""`
            "--arg=\"wrapped space\"", // `./my-app --args=""wrapped space""`
            "--arg=midword\"wrapped space\"", // `./my-app --args=midword""wrapped""`
            "",            // `./my-app '""'`
        ];
        // Note: These may not be the results we actually want (monitor this!).
        // We only make sure the implementation doesn't unintentionally change.
        let cases_escaped = [
            "something",
            "--flag",
            "--empty=",
            "--arg=value",
            "\"some space\"",
            "\"--arg value\"",
            "\"--arg=unwrapped space\"",
            "--arg=\\\"wrapped\\\"",
            "\"--arg=\\\"wrapped space\\\"\"",
            "\"--arg=midword\\\"wrapped space\\\"\"",
            "\"\"",
        ];

        // Just to be sure we didn't mess that up
        assert_eq!(cases.len(), cases_escaped.len());

        for (orig, escaped) in cases.iter().zip(cases_escaped) {
            assert_eq!(escape_nsis_current_exe_arg(&OsStr::new(orig)), escaped);
        }
    }

    /// 中文参数不含空格/斜杠 → 原样返回
    #[test]
    #[cfg(target_os = "windows")]
    fn it_escapes_chinese_arg_without_space() {
        assert_eq!(
            escape_nsis_current_exe_arg(OsStr::new("--name=张三")),
            OsStr::new("--name=张三")
        );
    }

    /// 中文 + 空格 → 整体加引号
    #[test]
    #[cfg(target_os = "windows")]
    fn it_escapes_chinese_arg_with_space() {
        assert_eq!(
            escape_nsis_current_exe_arg(OsStr::new("--备注=假期 作业")),
            OsStr::new("\"--备注=假期 作业\"")
        );
    }

    /// 长路径（含反斜杠分隔 + 空格）→ 加引号，路径内部反斜杠不倍增（2n 规则只在引号前生效）
    #[test]
    #[cfg(target_os = "windows")]
    fn it_escapes_long_path_with_space() {
        assert_eq!(
            escape_nsis_current_exe_arg(OsStr::new("C:\\Program Files\\My App\\data")),
            OsStr::new("\"C:\\Program Files\\My App\\data\"")
        );
    }

    /// 空格 + 结尾反斜杠 → 引号前补足 2n 个反斜杠（这里 n=1 → 2 个）
    #[test]
    #[cfg(target_os = "windows")]
    fn it_escapes_trailing_backslashes_before_quote() {
        assert_eq!(
            escape_nsis_current_exe_arg(OsStr::new("C:\\path with space\\")),
            OsStr::new("\"C:\\path with space\\\\\"")
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn compose_args_passive_with_restart() {
        let args = compose_nsis_args(
            NsisInstallMode::Passive,
            true,
            &[OsString::from("--port"), OsString::from("8080")],
            &[],
        );
        assert_eq!(args, OsString::from("/P /UPDATE /R /ARGS --port 8080"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn compose_args_silent_without_restart() {
        let args = compose_nsis_args(NsisInstallMode::Silent, false, &[OsString::from("--flag")], &[]);
        assert_eq!(args, OsString::from("/S /UPDATE"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn compose_args_basic_ui_with_custom_args() {
        let args = compose_nsis_args(
            NsisInstallMode::BasicUi,
            false,
            &[],
            &[OsString::from("--custom"), OsString::from("--extra=x")],
        );
        assert_eq!(args, OsString::from("/UPDATE --custom --extra=x"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn compose_args_restart_escapes_each_current_arg() {
        // restart 分支会把原启动参数逐项转义（含空格的项加引号）
        let args = compose_nsis_args(
            NsisInstallMode::Passive,
            true,
            &[OsString::from("--arg"), OsString::from("some space")],
            &[],
        );
        assert_eq!(args, OsString::from("/P /UPDATE /R /ARGS --arg \"some space\""));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn shell_execute_result_check() {
        assert!(check_shell_execute_result(33).is_ok());
        assert!(check_shell_execute_result(32).is_err());
        assert!(check_shell_execute_result(0).is_err());
        assert!(check_shell_execute_result(-1).is_err());
    }
}
