use crate::config::app_paths;
use crate::windows::common::WindowKind;
use anyhow::Context;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_decorum::WebviewWindowExt;
use tauri_plugin_prevent_default::PreventDefault;

/// 打开 `help` 窗口
pub fn open(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let kind = WindowKind::Help;
    match app.get_webview_window(kind.as_str()) {
        Some(help_window) => {
            help_window.unminimize().context("Failed to unminimize window")?;
            help_window.show().context("Failed to show window.")?;
            help_window.set_focus().context("Failed to focus window.")?;
        }
        None => {
            let help_window = WebviewWindowBuilder::new(app, kind.as_str(), WebviewUrl::App(kind.html().into()))
                .data_directory(app_paths::webview2_dir().to_path_buf())
                .inner_size(800.0, 600.0)
                .auto_resize()
                .title(kind.as_str())
                .decorations(false)
                .center()
                .initialization_script(app.prevent_default_script().to_string())
                .build()
                .context("Failed to build help window.")?;
            help_window
                .create_overlay_titlebar()
                .context("Failed to create overlay titlebar.")?;
        }
    };
    Ok(())
}
