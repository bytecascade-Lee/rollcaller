use crate::config::app_paths;
use anyhow::{anyhow, Context};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

pub fn open(app: tauri::AppHandle) -> anyhow::Result<()> {
    match app.get_webview_window("help") {
        Some(help_window) => {
            help_window.show().context("Failed to show window.")?;
            help_window.set_focus().context("Failed to focus window.")?;
        }
        None => {
            WebviewWindowBuilder::new(&app, "help", WebviewUrl::App("help.html".into()))
                .data_directory(app_paths::webview2_dir().to_path_buf())
                .inner_size(800.0, 600.0)
                .auto_resize()
                .center()
                .decorations(true)
                .title("Rollcaller Help")
                .build()
                .context("Failed to build window.")?;
        }
    };
    Ok(())
}

pub fn hide(app: tauri::AppHandle) -> anyhow::Result<()> {
    match app.get_webview_window("help") {
        Some(help_window) => Ok(help_window.hide().context("Failed to hide window.")?),
        None => Err(anyhow!("Couldnt find help window so we cant hide it.")),
    }
}

pub fn close(app: tauri::AppHandle) -> anyhow::Result<()> {
    match app.get_webview_window("help") {
        Some(help_window) => Ok(help_window.close().context("Failed to close window.")?),
        None => Err(anyhow!("Couldnt find help window so we cant close it.")),
    }
}

pub fn destroy(app: tauri::AppHandle) -> anyhow::Result<()> {
    match app.get_webview_window("help") {
        Some(help_window) => Ok(help_window.destroy().context("Failed to destroy window.")?),
        None => Err(anyhow!("Couldnt find help window so we cant destroy it.")),
    }
}
