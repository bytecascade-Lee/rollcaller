use crate::config::app_paths;
use std::fs;

#[tauri::command]
pub async fn help_load_markdown(id: String) -> Result<String, String> {
    fs::read_to_string(&app_paths::resources_dir().join(format!("help/{0}/{0}-zh-CN.md", id))).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn help_load_readme() -> Result<String, String> {
    Ok(fs::read_to_string(&app_paths::resources_dir().parent().unwrap().join("README.md"))
        .map_err(|e| e.to_string())?
        .replace("> [en-US](README_en_US.md)", ""))
}

#[tauri::command]
pub async fn help_load_license() -> Result<String, String> {
    Ok(fs::read_to_string(&app_paths::resources_dir().parent().unwrap().join("LICENSE"))
        .map_err(|e| e.to_string())?
        .replace("MIT License", "### MIT License")
        .replace("Copyright (c) 2026 Serene Lee", "*Copyright (c) 2026 Serene Lee*"))
}

#[tauri::command]
pub async fn help_load_changelog() -> Result<String, String> {
    fs::read_to_string(&app_paths::resources_dir().parent().unwrap().join("CHANGELOG.md")).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn help_load_release_notes() -> Result<String, String> {
    fs::read_to_string(&app_paths::resources_dir().parent().unwrap().join("RELEASE_NOTES.md")).map_err(|e| e.to_string())
}
