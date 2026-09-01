//! NSIS 安装器执行（Windows 专用）
//!
//! 流程（与 tauri-plugin-updater v2 的 `install_inner` 一致）：
//!
//! 1. 载荷识别：zip → 解压找 `.exe`；裸 `.exe` → 写临时文件；其他 → 报错；
//! 2. 退出前钩子（清理单实例锁等）；
//! 3. 构造安装器命令行（`install_mode 参数 /UPDATE [/R /ARGS <原启动参数>] [自定义参数]`）；
//! 4. `ShellExecuteW` 启动安装器，成功后 `exit(0)`，由安装器负责重启应用。
//!
//! 只做 NSIS，不做 MSI/WiX。实现搬运自插件源码（`updater-src/plugins/updater/src/updater.rs`），
//! 各函数注释中标注了源行号，便于日后对照升级。转义函数不得自创/简化。

use std::ffi::{OsStr, OsString};
use std::io::Cursor;
use std::path::PathBuf;

/// 临时目录前缀中的应用名与版本号
///
/// 插件端来自 `context.app_name` / `version`（清单中的目标应用与版本）；
/// 本模块暂无清单上下文，先用包信息占位，后续接入更新管线时再注入。
const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// NSIS 安装界面模式（对应插件 `WindowsUpdateInstallMode` 的 NSIS 分支，config.rs:14-23）
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

impl NsisInstallMode {
    /// 界面模式对应的 NSIS 参数（与插件 config.rs:41-50 一致）
    ///
    /// - `Passive` → `/P`
    /// - `Silent`  → `/S`
    /// - `BasicUi` → 无
    pub fn nsis_args(&self) -> &'static [&'static str] {
        match self {
            Self::Passive => &["/P"],
            Self::Silent => &["/S"],
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
pub struct NsisOptions {
    /// 安装界面模式（默认 Passive）
    pub install_mode: NsisInstallMode,
    /// 是否在安装完成后重启应用并继承原启动参数（默认 true）
    pub restart_after_install: bool,
    /// 启动安装器前的钩子（清理单实例锁等），成功启动后进程即退出
    ///
    /// 采用 `FnOnce`：钩子只应执行一次；因此 `install_nsis` 按值接收 `NsisOptions`，
    /// 与规格 2.4 的示例保持一致（§3 接口契约写的是 `&NsisOptions`，二者不可兼得，
    /// 取舍为保留 `FnOnce` 语义 + 按值传参；成功后 `exit(0)`，调用方本就不再使用 opts）。
    pub on_before_exit: Option<Box<dyn FnOnce() + Send>>,
    /// 用户自定义安装器参数（追加在命令行末尾；来源待配置化，本任务先留空）
    pub installer_args: Vec<OsString>,
}

impl Default for NsisOptions {
    fn default() -> Self {
        Self {
            install_mode: NsisInstallMode::Passive,
            restart_after_install: true,
            on_before_exit: None,
            installer_args: Vec::new(),
        }
    }
}

/// 安装入口：识别载荷 → 退出前钩子 → 构造参数 → `ShellExecuteW` 启动 → `exit(0)`
///
/// 对应插件 `install_inner`（updater.rs:835-877）。`bytes` 应为已验证（sha256 + 签名）的
/// 安装包字节，验签是 03 的职责，本函数不再校验。
///
/// 失败（如 `ShellExecuteW` 返回值 <= 32）返回 `Err`，**不会** `exit(0)`，交由上层决定；
/// 成功路径下安装器接管进程重启，本进程正常流程不可到达 `exit(0)` 之后。
#[cfg(windows)]
pub fn install_nsis(bytes: &[u8], mut opts: NsisOptions) -> anyhow::Result<()> {
    use windows_sys::{
        w,
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOW},
    };

    // 1. 载荷识别/解压（规格 2.1）
    let installer_path = extract(bytes)?;

    // 2. 退出前钩子（插件 install_inner 在启动安装器前调用，updater.rs:843-846）
    if let Some(hook) = opts.on_before_exit.take() {
        hook();
    }

    // 3. 构造安装器命令行（规格 2.2）
    let args = build_nsis_args(&opts);
    tracing::debug!("Executing updater {:?} with parameters: {:?}", installer_path, args);

    // 4. ShellExecuteW 启动，安装器接管后当前进程退出
    let file = encode_wide(&installer_path);
    let parameters = encode_wide(&args);
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
    check_shell_execute_result(result as isize)?;

    // 安装器接管（自行重启应用），当前进程立即退出
    std::process::exit(0);
}

/// 非 Windows 平台：NSIS 安装不可用（桩）
#[cfg(not(windows))]
pub fn install_nsis(_bytes: &[u8], _opts: NsisOptions) -> anyhow::Result<()> {
    anyhow::bail!("NSIS 安装仅支持 Windows")
}

/// 载荷识别：zip → 解压找 `.exe`；裸 `.exe` → 写临时文件；其他 → `InvalidUpdaterFormat`
///
/// 对应插件 `extract`（updater.rs:947-954）与 `extract_exe`（988-998），仅保留 NSIS 分支。
#[cfg(windows)]
fn extract(bytes: &[u8]) -> anyhow::Result<PathBuf> {
    if infer::archive::is_zip(bytes) {
        extract_zip(bytes)
    } else if infer::app::is_exe(bytes) {
        write_to_temp(bytes, ".exe")
    } else {
        anyhow::bail!("无法识别的更新载荷格式（非 zip 非 exe，InvalidUpdaterFormat）")
    }
}

/// 解压 zip 到临时目录并查找 `.exe` 安装器（对应插件 `extract_zip`，updater.rs:967-986）
#[cfg(windows)]
fn extract_zip(bytes: &[u8]) -> anyhow::Result<PathBuf> {
    let temp_dir = make_temp_dir()?;

    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    archive.extract(&temp_dir)?;

    for entry in std::fs::read_dir(&temp_dir)? {
        let path = entry?.path();
        if path.extension() == Some(OsStr::new("exe")) {
            return Ok(path);
        }
    }

    anyhow::bail!("ZIP 内未找到 .exe 安装器（BinaryNotFoundInArchive）")
}

/// 把裸 `.exe` 字节写入临时文件并返回路径（对应插件 `write_to_temp`，updater.rs:1000-1020）
///
/// `TempPath::keep()` 防止文件在 drop 时被删除——它必须存活到 `ShellExecuteW` 执行完。
#[cfg(windows)]
fn write_to_temp(bytes: &[u8], ext: &str) -> anyhow::Result<PathBuf> {
    use std::io::Write;

    let temp_dir = make_temp_dir()?;
    let mut temp_file = tempfile::Builder::new()
        .prefix(&format!("{APP_NAME}-{APP_VERSION}-installer"))
        .suffix(ext)
        .rand_bytes(0)
        .tempfile_in(&temp_dir)?;
    temp_file.write_all(bytes)?;
    Ok(temp_file.into_temp_path().keep()?)
}

/// 创建本次更新专用的临时目录（命名 `{app}-{version}-updater-`，对应插件 `make_temp_dir`，
/// updater.rs:956-964）；目录保留不自动清理，由安装器/系统临时目录回收
#[cfg(windows)]
fn make_temp_dir() -> anyhow::Result<PathBuf> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("{APP_NAME}-{APP_VERSION}-updater-"))
        .tempdir_in(temp_dir_root())?
        .keep())
}

/// 临时目录根：优先复用应用自有 `app_paths::temp_dir()`（规格 2.1），否则回退系统 temp
#[cfg(windows)]
fn temp_dir_root() -> PathBuf {
    let app_temp = crate::config::app_paths::temp_dir();
    if std::fs::create_dir_all(app_temp).is_ok() {
        app_temp.to_path_buf()
    } else {
        std::env::temp_dir()
    }
}

/// 构造传给 NSIS 安装器的完整命令行参数
///
/// 对应插件 `updater_parameters` 的 NSIS 分支（updater.rs:879-909）：
/// `<installMode 参数> /UPDATE [/R /ARGS <转义后的原启动参数...>] [用户自定义 installer_args]`
#[cfg(windows)]
pub(crate) fn build_nsis_args(opts: &NsisOptions) -> OsString {
    let current_exe_args: Vec<OsString> = std::env::args_os().skip(1).collect();
    compose_nsis_args(
        opts.install_mode,
        opts.restart_after_install,
        &current_exe_args,
        &opts.installer_args,
    )
}

/// 参数拼装的纯函数（便于单测）：不读进程环境，所有输入显式传入
#[cfg(windows)]
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
        escaped = current_exe_args
            .iter()
            .map(escape_nsis_current_exe_arg)
            .collect();
        parts.extend(
            install_mode
                .nsis_restart_after_install_args()
                .iter()
                .map(OsStr::new),
        );
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
#[cfg(windows)]
fn escape_nsis_current_exe_arg(arg: impl AsRef<OsStr>) -> OsString {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    let arg = arg.as_ref();
    let mut cmd: Vec<u16> = Vec::new();

    // compared to std we additionally escape `/` so that nsis won't interpret them as a beginning of an nsis argument.
    let quote = arg
        .as_encoded_bytes()
        .iter()
        .any(|c| *c == b' ' || *c == b'\t' || *c == b'/')
        || arg.is_empty();
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
#[cfg(windows)]
fn encode_wide(string: impl AsRef<OsStr>) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    string
        .as_ref()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// 校验 `ShellExecuteW` 返回值：> 32 表示成功，其余为错误（与插件 install_inner 判定一致，
/// updater.rs:872-874）。抽出为独立函数便于单测该分支。
#[cfg(windows)]
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

    /// 构造仅含一个文件的最小 zip（Stored 不压缩，便于内容比对）
    #[cfg(windows)]
    fn make_test_zip(file_name: &str, content: &[u8]) -> Vec<u8> {
        use std::io::Write;

        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            zip.start_file(
                file_name,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .expect("写入 zip 条目失败");
            zip.write_all(content).expect("写入 zip 内容失败");
            zip.finish().expect("结束 zip 写入失败");
        }
        buf.into_inner()
    }

    // ------------------------------------------------------------------
    // 转义函数：11 组用例原样搬运自插件 updater.rs:1700-1741（it_escapes_correctly_for_nsis）
    // ------------------------------------------------------------------
    #[test]
    #[cfg(windows)]
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

    // ------------------------------------------------------------------
    // 转义函数：补充中文 / 长路径用例（规格 5 要求）
    // ------------------------------------------------------------------

    /// 中文参数不含空格/斜杠 → 原样返回
    #[test]
    #[cfg(windows)]
    fn it_escapes_chinese_arg_without_space() {
        assert_eq!(
            escape_nsis_current_exe_arg(OsStr::new("--name=张三")),
            OsStr::new("--name=张三")
        );
    }

    /// 中文 + 空格 → 整体加引号
    #[test]
    #[cfg(windows)]
    fn it_escapes_chinese_arg_with_space() {
        assert_eq!(
            escape_nsis_current_exe_arg(OsStr::new("--备注=假期 作业")),
            OsStr::new("\"--备注=假期 作业\"")
        );
    }

    /// 长路径（含反斜杠分隔 + 空格）→ 加引号，路径内部反斜杠不倍增（2n 规则只在引号前生效）
    #[test]
    #[cfg(windows)]
    fn it_escapes_long_path_with_space() {
        assert_eq!(
            escape_nsis_current_exe_arg(OsStr::new("C:\\Program Files\\My App\\data")),
            OsStr::new("\"C:\\Program Files\\My App\\data\"")
        );
    }

    /// 空格 + 结尾反斜杠 → 引号前补足 2n 个反斜杠（这里 n=1 → 2 个）
    #[test]
    #[cfg(windows)]
    fn it_escapes_trailing_backslashes_before_quote() {
        assert_eq!(
            escape_nsis_current_exe_arg(OsStr::new("C:\\path with space\\")),
            OsStr::new("\"C:\\path with space\\\\\"")
        );
    }

    // ------------------------------------------------------------------
    // 载荷识别：zip 内嵌 exe / 裸 exe / 无效字节 / zip 无 exe（规格 5 要求）
    // ------------------------------------------------------------------

    #[test]
    #[cfg(windows)]
    fn extract_detects_zip_with_exe() {
        let content = b"MZ\x90\x00\x00fake-exe";
        let zip_bytes = make_test_zip("setup.exe", content);

        let path = extract(&zip_bytes).expect("zip 内嵌 exe 应识别成功");
        assert_eq!(path.extension(), Some(OsStr::new("exe")));
        assert_eq!(std::fs::read(&path).expect("读取落盘安装器失败"), content);
    }

    #[test]
    #[cfg(windows)]
    fn extract_detects_bare_exe() {
        let content = b"MZ\x90\x00\x00fake-exe";

        let path = extract(content).expect("裸 exe 应识别成功");
        assert_eq!(path.extension(), Some(OsStr::new("exe")));
        assert_eq!(std::fs::read(&path).expect("读取落盘安装器失败"), content);
    }

    #[test]
    #[cfg(windows)]
    fn extract_rejects_invalid_bytes() {
        assert!(extract(b"not a zip or exe").is_err());
    }

    #[test]
    #[cfg(windows)]
    fn extract_rejects_zip_without_exe() {
        let zip_bytes = make_test_zip("readme.txt", b"hello");
        assert!(extract(&zip_bytes).is_err());
    }

    // ------------------------------------------------------------------
    // 参数拼装：passive / silent / basicUi、restart 开关、自定义 args 顺序（规格 5 要求）
    // ------------------------------------------------------------------

    #[test]
    #[cfg(windows)]
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
    #[cfg(windows)]
    fn compose_args_silent_without_restart() {
        let args = compose_nsis_args(
            NsisInstallMode::Silent,
            false,
            &[OsString::from("--flag")],
            &[],
        );
        assert_eq!(args, OsString::from("/S /UPDATE"));
    }

    #[test]
    #[cfg(windows)]
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
    #[cfg(windows)]
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

    // ------------------------------------------------------------------
    // ShellExecuteW 返回值判定（规格 5 要求：抽函数测 <=32 返回 Err 分支）
    // ------------------------------------------------------------------

    #[test]
    #[cfg(windows)]
    fn shell_execute_result_check() {
        assert!(check_shell_execute_result(33).is_ok());
        assert!(check_shell_execute_result(32).is_err());
        assert!(check_shell_execute_result(0).is_err());
        assert!(check_shell_execute_result(-1).is_err());
    }

    /// on_before_exit 钩子在按值接收的 opts 中只执行一次
    #[test]
    #[cfg(windows)]
    fn on_before_exit_runs_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = std::sync::Arc::new(AtomicUsize::new(0));
        let hook_counter = counter.clone();
        let opts = NsisOptions {
            on_before_exit: Some(Box::new(move || {
                hook_counter.fetch_add(1, Ordering::SeqCst);
            })),
            ..Default::default()
        };

        let mut opts = opts;
        if let Some(hook) = opts.on_before_exit.take() {
            hook();
        }
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
