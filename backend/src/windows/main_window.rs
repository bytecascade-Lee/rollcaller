use crate::config::app_paths;
use anyhow::{anyhow, Context};
use tauri::{Manager, WebviewWindowBuilder};

pub fn init(app: &mut tauri::App) -> anyhow::Result<()> {
    let _ = WebviewWindowBuilder::new(app, "main", Default::default())
        .data_directory(app_paths::webview_dir().to_path_buf())
        .inner_size(900.0, 700.0)
        .auto_resize()
        .center()
        .decorations(true)
        .title("Rollcaller")
        .build();
    Ok(())
}

pub fn open(app: tauri::AppHandle) -> anyhow::Result<()> {
    match app.get_webview_window("main") {
        Some(main_window) => {
            main_window
                .show()
                .context("Failed to show window.")?;
            main_window
                .set_focus()
                .context("Failed to focus window.")?;
            Ok(())
        }
        None => Err(anyhow!("竟然没有main标签的窗口？？？是不是打成mian了？")),
    }
}
