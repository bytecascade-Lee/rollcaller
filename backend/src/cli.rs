//! CLI 场景处理：`-v/--version`、`--build-info`、`--app-paths`、`--mode`

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
