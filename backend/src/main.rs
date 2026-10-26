#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    rollcaller_lib::console::attach();
    rollcaller_lib::cli::handle_cli_args();
    tauri::async_runtime::block_on(rollcaller_lib::run());
}
