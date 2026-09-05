//! 更新管线（自研，替代前端 tauri-plugin-updater 驱动）——Tauri 侧编排模块
//!
//! 领域实现（check / download / verify / install / version）已收敛到
//! `service/update`，本模块只保留"贴近 Tauri 运行时"的部分：
//!
//! - [`commands`]：更新命令（check / download / cancel / install），防重入 + 事件上报；
//! - [`state`]：更新会话（PendingUpdate）+ 防重入守卫；

pub mod commands;
pub mod state;
