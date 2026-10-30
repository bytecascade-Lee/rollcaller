//! CLI 场景：`-h/--help`、`-v/--version`、`--build-info`、`--app-paths`、`--mode`、`--doctor`

mod args;
mod doctor;

use crate::config::{app_info, app_paths};
use args::{Command, Format, Invocation};
use serde::Serialize;
use serde_json::json;
use std::process::exit;

/// 用法错误的指路行（不重复整份帮助）
const USAGE_HINT: &str = "用法：rollcaller <命令> [--json]，详见 `rollcaller --help`";

const HELP: &str = "\
rollcaller — 课堂自动点名

用法：
  rollcaller                启动图形界面
  rollcaller <命令> [--json]

命令：
  -h, --help        显示本帮助
  -v, --version     版本号
      --build-info  构建信息：版本 / 提交 / 构建时间 / 目标平台
      --app-paths   应用路径：运行形态 / 根目录 / 数据 / 缓存 / 日志 ...
      --mode        运行形态：Develop / Portable / Install
      --doctor      诊断信息：构建事实 + 路径 + 自检

选项：
      --json        输出 JSON

环境变量：
  ROLLCALLER_ATTACH_CONSOLE_ON_WIN32=1
                    在终端里启动图形界面时，把日志打到该终端，
                    仅限 Windows 平台，默认关闭，0 / false / off / 空 视为关闭

退出码：0 成功 · 2 用法错误 · 1 运行失败\n";

/// 处理 CLI 参数：命中命令打印后退出，未命中静默返回由调用方继续启动窗口
pub fn handle_cli_args() {
    match args::parse(std::env::args().skip(1)) {
        // 没有 CLI 意图
        Ok(None) => {}
        Ok(Some(invocation)) => {
            crate::console::attach_for_cli();
            dispatch(invocation);
        }
        Err(e) => {
            crate::console::attach_for_cli();
            eprintln!("rollcaller: {e}");
            eprintln!("{USAGE_HINT}");
            exit(2);
        }
    }
}

/// 打印命令结果并退出
fn dispatch(invocation: Invocation) -> ! {
    if let Err(e) = run(invocation) {
        eprintln!("rollcaller: {e}");
        exit(1);
    }
    exit(0)
}

/// 执行一次 CLI 调用
fn run(invocation: Invocation) -> Result<(), String> {
    let json = invocation.format == Format::Json;
    match invocation.command {
        Command::Help => println!("{HELP}"),
        Command::Version => {
            let version = app_info::app_info().version.to_string();
            if json {
                print_json(&json!({ "version": version }))?;
            } else {
                println!("{version}");
            }
        }
        Command::Mode => {
            let mode = app_paths::current_mode().to_string();
            if json {
                print_json(&json!({ "mode": mode }))?;
            } else {
                println!("{mode}");
            }
        }
        Command::BuildInfo => {
            if json {
                print_json(app_info::app_info())?;
            } else {
                print_rows(&build_info_rows());
            }
        }
        Command::AppPaths => {
            if json {
                print_json(app_paths::paths())?;
            } else {
                print_rows(&app_paths_rows());
            }
        }
        Command::Doctor => doctor::print(json)?,
    }
    Ok(())
}

/// 打印对齐的 `key : value`（键宽按本组最长键）
fn print_rows(rows: &[(&str, String)]) {
    let width = rows.iter().map(|(key, _)| key.chars().count()).max().unwrap_or(0);
    for (key, value) in rows {
        println!("{key:<width$} : {value}", width = width);
    }
}

/// 打印单个 JSON 对象（`--json` 的唯一输出）
fn print_json<T: Serialize>(value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| format!("序列化失败：{e}"))?;
    println!("{text}");
    Ok(())
}

/// 构建信息的人类可读视图
fn build_info_rows() -> Vec<(&'static str, String)> {
    use crate::util::time_utils::ISO8601;

    let info = app_info::app_info();
    let build = &info.build_info;
    vec![
        ("version", info.version.to_string()),
        ("target", info.target.clone()),
        ("os", info.os.to_string()),
        ("arch", info.arch.to_string()),
        ("ref_name", build.ref_name.clone()),
        ("commit_count", build.commit_count.to_string()),
        ("commit_hash", build.commit_hash.clone()),
        ("abbreviated_hash", build.abbreviated_hash.clone()),
        ("commit_time", ISO8601(&build.commit_time).to_string()),
        ("build_time", ISO8601(&build.build_time).to_string()),
    ]
}

/// 应用路径的人类可读视图（键名与 `--json` 的 `AppPaths` 一致）
fn app_paths_rows() -> Vec<(&'static str, String)> {
    vec![
        ("mode", app_paths::current_mode().to_string()),
        ("root_dir", app_paths::root_dir().display().to_string()),
        ("config_dir", app_paths::config_dir().display().to_string()),
        ("data_dir", app_paths::data_dir().display().to_string()),
        ("cache_dir", app_paths::cache_dir().display().to_string()),
        ("temp_dir", app_paths::temp_dir().display().to_string()),
        ("logs_dir", app_paths::logs_dir().display().to_string()),
        ("webview_dir", app_paths::webview2_dir().display().to_string()),
        ("resources_dir", app_paths::resources_dir().display().to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted_keys(value: serde_json::Value) -> Vec<String> {
        let mut keys: Vec<String> = value.as_object().expect("应为 JSON 对象").keys().cloned().collect();
        keys.sort_unstable();
        keys
    }

    fn sorted_row_keys(rows: &[(&'static str, String)]) -> Vec<String> {
        let mut keys: Vec<String> = rows.iter().map(|(key, _)| key.to_string()).collect();
        keys.sort_unstable();
        keys
    }

    /// 人类可读视图与 `--json` 必须是同一份数据，键名漂移即红
    #[test]
    fn human_keys_match_json_keys() {
        let info = serde_json::to_value(app_info::app_info()).unwrap();
        assert_eq!(sorted_row_keys(&build_info_rows()), sorted_keys(info));

        let paths = serde_json::to_value(app_paths::paths()).unwrap();
        assert_eq!(sorted_row_keys(&app_paths_rows()), sorted_keys(paths));
    }

    /// 帮助里必须出现环境变量全名
    #[test]
    fn help_documents_attach_env() {
        assert!(
            HELP.contains(crate::console::ATTACH_ENV),
            "帮助文本缺少环境变量 {}",
            crate::console::ATTACH_ENV
        );
    }
}
