use crate::config::app_paths;
use std::fs;

const LICENSE: &str = include_str!("../../../LICENSE");

/// README 中只在 GitHub 展示的区块标记：帮助窗口渲染不了仓库门面的图片与跨文档链接。
const GITHUB_ONLY_BEGIN: &str = "<!-- help:hide -->";
const GITHUB_ONLY_END: &str = "<!-- help:show -->";

#[tauri::command]
pub async fn help_load_markdown(id: String) -> Result<String, String> {
    fs::read_to_string(&app_paths::resources_dir().join(format!("help/{0}/{0}-zh-CN.md", id))).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn help_load_readme() -> Result<String, String> {
    let markdown = fs::read_to_string(&app_paths::root_dir().join("README.md")).map_err(|e| e.to_string())?;
    Ok(strip_github_only(&markdown))
}

#[tauri::command]
pub async fn help_load_license() -> Result<String, String> {
    Ok(String::from(LICENSE))
}

#[tauri::command]
pub async fn help_load_changelog() -> Result<String, String> {
    fs::read_to_string(&app_paths::root_dir().join("CHANGELOG.md")).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn help_load_release_notes() -> Result<String, String> {
    fs::read_to_string(&app_paths::root_dir().join("RELEASE_NOTES.md")).map_err(|e| e.to_string())
}

/// 丢弃 `<!-- help:hide -->` 与 `<!-- help:show -->` 之间的内容（含标记本身），并去掉首尾空行
fn strip_github_only(markdown: &str) -> String {
    let mut kept: Vec<&str> = vec!["# Rollcaller", "---"];
    let mut hiding = false;
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed == GITHUB_ONLY_BEGIN {
            hiding = true;
        } else if trimmed == GITHUB_ONLY_END {
            hiding = false;
        } else if !hiding {
            kept.push(line);
        }
    }
    kept.join("\n").trim_matches('\n').to_string()
}
