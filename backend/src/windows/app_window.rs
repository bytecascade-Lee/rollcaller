use crate::config::app_paths;
use anyhow::Context;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_decorum::WebviewWindowExt;
use tauri_plugin_prevent_default::PreventDefault;

pub fn open(app: tauri::AppHandle) -> anyhow::Result<()> {
    match app.get_webview_window("app") {
        Some(app_window) => {
            app_window.unminimize().context("Failed to unminimize window")?;
            app_window.show().context("Failed to show window.")?;
            app_window.set_focus().context("Failed to focus window.")?;
        }
        None => {
            let app_window = WebviewWindowBuilder::new(&app, "app", WebviewUrl::App("app.html".into()))
                .data_directory(app_paths::webview2_dir().to_path_buf())
                .inner_size(900.0, 700.0)
                .auto_resize()
                .title("app")
                .decorations(false)
                .center()
                .initialization_script(app.prevent_default_script().to_string())
                .build()
                .context("Failed to build app window.")?;
            app_window.create_overlay_titlebar().context("Failed to create overlay titlebar.")?;
        }
    };
    Ok(())
}
