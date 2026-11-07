//! 平台实现选择：按目标 OS 二选一，唯一的 `#[cfg]` 站点。
//!
//! 新增平台 = 新增 `platform/<os>.rs`（或 `<os>/` 目录）并在这里加一组 `mod` / `pub use`。

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::launch;

#[cfg(not(target_os = "windows"))]
mod unsupported;
#[cfg(not(target_os = "windows"))]
pub use unsupported::launch;
