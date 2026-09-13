//! CLI 场景处理：`-v/--version`、`--build-info`、`--app-paths`、`--mode`
//!
//! 命中任一参数时附着父进程控制台、打印结果并退出进程（不再返回），
//! 完全不进入 Tauri 运行时——无窗口、无日志初始化、无数据库迁移。
//! 未命中参数时静默返回，正常走窗口启动流程。

use crate::config::{app_info, app_paths};
use std::process::exit;

/// 处理 CLI 参数：多个参数共存时按第一个命中的输出并退出
pub fn handle_cli_args() {
    if let Some(arg) = std::env::args().skip(1).find(|a| {
        matches!(
            a.as_str(),
            "-v" | "--version" | "--build-info" | "--app-paths" | "--mode"
        )
    }) {
        attach_parent_console();
        match arg.as_str() {
            "-v" | "--version" => println!("{}", app_info::app_info().version),
            "--build-info" => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(app_info::app_info()).expect("AppInfo 序列化失败")
                )
            }
            "--app-paths" => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(app_paths::paths()).expect("AppPaths 序列化失败")
                )
            }
            "--mode" => println!("{}", app_paths::current_mode()),
            _ => unreachable!(),
        }
        exit(0);
    }
}

/// Windows GUI 子系统（windows_subsystem = "windows"）下进程不持有控制台，
/// 直接 println! 写入无效句柄不可见；附着父进程控制台后输出才能落在终端里。
/// debug 构建为 console 子系统天然有控制台，附着失败也无害，一律调用即可。
#[cfg(windows)]
fn attach_parent_console() {
    use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_parent_console() {}
