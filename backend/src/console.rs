//! 控制台通道
//!
//! Windows 模式下，release profile 以 GUI 子系统构建，系统不会为进程连接控制台；
//! 即使从 cmd / PowerShell 启动，`stdout` 也是无效句柄，`println!` 与 `tracing` 输出会被静默丢弃。
//!
//! 因此需要两步：
//!
//! 1. [`AttachConsole`](https://learn.microsoft.com/windows/console/attachconsole) 附着父进程控制台；
//! 2. 附着成功后重新打开 `CONOUT$`，并用 `SetStdHandle` 重新绑定 `stdout` / `stderr`。
//!
//! # 默认不接管终端
//!
//! Windows 上常规启动 [`attach`] 只有在 [`ATTACH_ENV`] 为真值时附着控制台；双击、自启动、服务等保持默认。
//!
//! # 平台差异
//!
//! Unix 没有子系统概念，也没有控制台句柄，进程直接继承父进程的 0/1/2，终端启动天然可见输出，
//! 桌面图标启动指向 `/dev/null`，静默丢弃。
//! 因此附着动作只存在于 Windows；Unix 侧仅判断「是否连接终端」以决定 ANSI。
//!
//! # 禁止
//!
//! - 不要使用 `AllocConsole`：双击启动时会凭空弹出黑窗；
//! - 不要把 `windows_subsystem` 改回 `console`：子系统决定 shell 是否等待，改回会在每次启动时
//!   闪一帧控制台窗口，对 release profile 下的用户来说体验极差。

use std::sync::OnceLock;

/// 输出通道状态（进程内只判定一次）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleState {
    /// 持有真正的控制台
    Attached,
    /// stdout 已被重定向到文件或管道，此时有输出通道，但没有终端语义
    Redirected,
    /// 没有任何输出通道，可能是双击启动、自启动或服务，此时写入被静默丢弃
    Detached,
}

impl ConsoleState {
    /// 是否存在可用输出通道（决定日志是否挂控制台层）
    pub fn has_output(self) -> bool {
        !matches!(self, Self::Detached)
    }

    /// 是否附着在真正的控制台上（决定 ANSI 与 Ctrl+C 语义）
    pub fn is_console(self) -> bool {
        matches!(self, Self::Attached)
    }
}

static STATE: OnceLock<ConsoleState> = OnceLock::new();

/// 置为真值时，从终端启动的 GUI 进程会附着父进程控制台
pub const ATTACH_ENV: &str = "ROLLCALLER_ATTACH_CONSOLE_ON_WIN32";

/// 常规启动：判定输出通道，必要时附着父进程控制台
///
/// 只有 [`ATTACH_ENV`] 为真值才去附着（见模块文档「默认不接管终端」）。
/// 必须在任何输出之前调用；幂等
pub fn attach() -> ConsoleState {
    *STATE.get_or_init(|| detect(requested()))
}

/// CLI 命令 / 用法错误，无条件附着
pub fn attach_for_cli() -> ConsoleState {
    debug_assert!(
        STATE.get().is_none(),
        "attach_for_cli 必须早于 attach：控制台通道在进程内只判定一次"
    );
    *STATE.get_or_init(|| detect(true))
}

/// 已判定的状态；尚未调用 [`attach`] 时按「无输出通道」处理
pub fn state() -> ConsoleState {
    STATE.get().copied().unwrap_or(ConsoleState::Detached)
}

/// 是否存在可用输出通道
pub fn has_output() -> bool {
    state().has_output()
}

/// 是否启用 ANSI 颜色。只有真控制台、且未被环境变量显式关闭时启用
pub fn ansi_enabled() -> bool {
    state().is_console() && color_allowed()
}

/// 遵循 `NO_COLOR` 与 `TERM=dumb`
fn color_allowed() -> bool {
    if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        return false;
    }
    std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true)
}

/// [`ATTACH_ENV`] 是否为真值
fn requested() -> bool {
    std::env::var_os(ATTACH_ENV).is_some_and(|value| is_truthy(&value.to_string_lossy()))
}

/// 真值判定
///
/// 未设置 / 空串 / `0` / `false` / `off` / `no` / `not` 为假，其余一律为真
fn is_truthy(raw: &str) -> bool {
    !matches!(raw.trim().to_ascii_lowercase().as_str(), "" | "0" | "false" | "off" | "no" | "not")
}

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HANDLE;

#[cfg(target_os = "windows")]
static CONOUT: OnceLock<std::fs::File> = OnceLock::new();

#[cfg(target_os = "windows")]
fn detect(requested: bool) -> ConsoleState {
    use windows_sys::Win32::Storage::FileSystem::{GetFileType, FILE_TYPE_DISK, FILE_TYPE_PIPE};
    use windows_sys::Win32::System::Console::{
        AttachConsole, GetConsoleMode, GetStdHandle, ATTACH_PARENT_PROCESS, STD_OUTPUT_HANDLE,
    };

    unsafe {
        let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode = 0;
        let already = GetConsoleMode(stdout, &mut mode) != 0;
        let redirected = matches!(GetFileType(stdout), FILE_TYPE_DISK | FILE_TYPE_PIPE);

        let attached = if already {
            // debug / develop 是 console 子系统，创建即附着，只需补开 VT 处理
            enable_vt(stdout, mode);
            true
        } else if redirected || !requested {
            false
        } else {
            AttachConsole(ATTACH_PARENT_PROCESS) != 0 && rebind_stdout()
        };

        classify(already, redirected, attached)
    }
}

/// 把系统调用的结果映射为状态
#[cfg(target_os = "windows")]
fn classify(already_attached: bool, redirected: bool, attach_ok: bool) -> ConsoleState {
    if already_attached {
        ConsoleState::Attached
    } else if redirected {
        ConsoleState::Redirected
    } else if attach_ok {
        ConsoleState::Attached
    } else {
        ConsoleState::Detached
    }
}

/// 把新打开的控制台句柄接到 `stdout` / `stderr` 上
///
/// Rust 的 `std::io::stdout()` 每次写入都重新 `GetStdHandle`，所以 `SetStdHandle`
/// 对 `println!`、`eprintln!` 与 `tracing` 同时生效，无需重开 CRT 的 `stdout`
#[cfg(target_os = "windows")]
fn rebind_stdout() -> bool {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Console::{GetConsoleMode, SetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};

    let Ok(file) = std::fs::OpenOptions::new().read(true).write(true).open("CONOUT$") else {
        return false;
    };
    let handle = file.as_raw_handle() as HANDLE;
    // 句柄活到进程结束：所有权移进全局，绝不能在这里 drop
    let _ = CONOUT.set(file);

    if unsafe { SetStdHandle(STD_OUTPUT_HANDLE, handle) } == 0 {
        return false;
    }
    unsafe { SetStdHandle(STD_ERROR_HANDLE, handle) };

    let mut mode = 0;
    if unsafe { GetConsoleMode(handle, &mut mode) } != 0 {
        enable_vt(handle, mode);
    }
    true
}

/// 打开虚拟终端序列处理
///
/// 旧 conhost 不支持时静默降级
#[cfg(target_os = "windows")]
fn enable_vt(handle: HANDLE, mode: u32) {
    use windows_sys::Win32::System::Console::{SetConsoleMode, ENABLE_VIRTUAL_TERMINAL_PROCESSING};
    unsafe { SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) };
}

#[cfg(not(target_os = "windows"))]
fn detect(_requested: bool) -> ConsoleState {
    use std::io::IsTerminal;
    // 桌面启动时 stdout 指向 /dev/null，写成 Redirected 即可
    if std::io::stdout().is_terminal() {
        ConsoleState::Attached
    } else {
        ConsoleState::Redirected
    }
}

/// 注册 Ctrl+C / Ctrl+Break 处理
///
/// 需要在 Tauri 运行时起来之后调用，回调里应使用异步运行时
#[cfg(target_os = "windows")]
pub fn install_interrupt_handler() {
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
    unsafe { SetConsoleCtrlHandler(Some(on_console_ctrl), 1) };
}

/// Unix 侧保持默认
#[cfg(not(target_os = "windows"))]
pub fn install_interrupt_handler() {}

/// 控制台回调
#[cfg(target_os = "windows")]
unsafe extern "system" fn on_console_ctrl(event: u32) -> i32 {
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT};
    match event {
        CTRL_C_EVENT | CTRL_BREAK_EVENT => {
            std::thread::spawn(shutdown_from_console);
            1
        }
        _ => 0,
    }
}

#[cfg(target_os = "windows")]
fn shutdown_from_console() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static INTERRUPTED: AtomicBool = AtomicBool::new(false);

    // 第二次 Ctrl+C 表示，不再执行收尾
    if INTERRUPTED.swap(true, Ordering::SeqCst) {
        std::process::exit(0);
    }

    tracing::info!("收到控制台中断信号，正在退出");
    tauri::async_runtime::block_on(crate::shutdown_hooks::finish_and_exit(0))
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn already_attached_wins() {
        assert_eq!(classify(true, false, false), ConsoleState::Attached);
        assert_eq!(classify(true, true, true), ConsoleState::Attached);
    }

    #[test]
    fn redirect_is_never_overridden_by_attach() {
        // 即使理论上能 attach，重定向也必须优先，否则 `> out.txt` 会被抢回控制台
        assert_eq!(classify(false, true, true), ConsoleState::Redirected);
        assert_eq!(classify(false, true, false), ConsoleState::Redirected);
    }

    #[test]
    fn no_console_means_detached() {
        assert_eq!(classify(false, false, true), ConsoleState::Attached);
        assert_eq!(classify(false, false, false), ConsoleState::Detached);
    }

    #[test]
    fn state_semantics() {
        assert!(ConsoleState::Attached.has_output() && ConsoleState::Attached.is_console());
        assert!(ConsoleState::Redirected.has_output() && !ConsoleState::Redirected.is_console());
        assert!(!ConsoleState::Detached.has_output() && !ConsoleState::Detached.is_console());
    }

    #[test]
    fn falsy_values_keep_console_detached() {
        for raw in ["", "   ", "0", "false", "FALSE", " off ", "Off", "no", "NO", "not", "nOt"] {
            assert!(!is_truthy(raw), "{raw:?} 应判定为关闭");
        }
    }

    #[test]
    fn any_other_value_turns_console_on() {
        for raw in ["1", "true", "ON", "yes", "abc", " 1 ", "0x0"] {
            assert!(is_truthy(raw), "{raw:?} 应判定为开启");
        }
    }
}
