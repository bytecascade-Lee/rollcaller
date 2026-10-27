//! `--doctor` 命令，输出全部支持信息

use super::{app_paths_rows, build_info_rows, print_json, print_rows};
use crate::config::{app_info, app_paths};
use crate::database::database_pool;
use serde::Serialize;
use std::fs;
use std::path::Path;

/// 一条自检结果
#[derive(Serialize)]
struct Check {
    /// 检查项名
    name: &'static str,
    /// 是否通过
    ok: bool,
    /// 具体细节
    detail: String,
}

/// `--json` 的完整报告
#[derive(Serialize)]
struct Report<'a> {
    app: &'a app_info::AppInfo,
    exe: String,
    paths: &'a app_paths::AppPaths,
    checks: Vec<Check>,
}

pub(super) fn print(json: bool) -> Result<(), String> {
    let checks = run_checks();

    if json {
        let report = Report {
            app: app_info::app_info(),
            exe: exe_path(),
            paths: app_paths::paths(),
            checks,
        };
        return print_json(&report);
    }

    let mut rows = build_info_rows();
    rows.push(("exe", exe_path()));
    rows.extend(app_paths_rows());
    print_rows(&rows);
    print_checks(&checks);
    Ok(())
}

fn print_checks(checks: &[Check]) {
    let width = checks.iter().map(|check| check.name.chars().count()).max().unwrap_or(0);
    println!();
    println!("自检：");
    for check in checks {
        let tag = if check.ok { "[ OK ]" } else { "[FAIL]" };
        println!("  {tag} {name:<width$} : {detail}", name = check.name, detail = check.detail);
    }
}

fn exe_path() -> String {
    std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|e| format!("<无法获取：{e}>"))
}

fn run_checks() -> Vec<Check> {
    vec![
        writable_dir("data_dir_writable", app_paths::data_dir()),
        writable_dir("logs_dir_writable", app_paths::logs_dir()),
        database_file(),
        webview2_dir(),
    ]
}

/// 目录可写探针
fn writable_dir(name: &'static str, dir: &Path) -> Check {
    if !dir.is_dir() {
        return Check {
            name,
            ok: false,
            detail: "目录不存在".to_string(),
        };
    }
    let probe = dir.join(".doctor-write-probe");
    let result = fs::write(&probe, b"");
    let _ = fs::remove_file(&probe);
    match result {
        Ok(()) => Check {
            name,
            ok: true,
            detail: "可写".to_string(),
        },
        Err(e) => Check {
            name,
            ok: false,
            detail: e.to_string(),
        },
    }
}

/// 数据库文件
///
/// - 存在 -> 说明大小
/// - 不存在 -> 首次启动前的正常状态
fn database_file() -> Check {
    let path = database_pool::db_path();
    match fs::metadata(&path) {
        Ok(meta) => Check {
            name: "database_file",
            ok: true,
            detail: format!("{}（{} 字节）", path.display(), meta.len()),
        },
        Err(_) => Check {
            name: "database_file",
            ok: false,
            detail: format!("{}（不存在，首次启动时创建）", path.display()),
        },
    }
}

/// WebView2 运行时目录
fn webview2_dir() -> Check {
    let dir = app_paths::webview2_dir();
    Check {
        name: "webview2_dir",
        ok: dir.is_dir(),
        detail: if dir.is_dir() {
            dir.display().to_string()
        } else {
            format!("{}（不存在）", dir.display())
        },
    }
}
