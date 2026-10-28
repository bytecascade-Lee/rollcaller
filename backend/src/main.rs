#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    rollcaller_lib::cli::handle_cli_args();
    // 运行到此处意味着是 GUI 启动，是否接管终端由环境变量 ROLLCALLER_ATTACH_CONSOLE_ON_WIN32 决定
    rollcaller_lib::console::attach();
    tauri::async_runtime::block_on(rollcaller_lib::run());
}
