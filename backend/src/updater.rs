//! 统一更新管线（自研，替代前端 tauri-plugin-updater 驱动）
//!
//! - 任务 01：manifest 解析与版本策略（纯数据层）
//! - 任务 02：check（拉取清单 + 版本判定）与 download（流式下载 + 进度）
//!
//! 本模块暂不涉及验签（任务 03）与安装（任务 04-06）。
//! 数据层类型与重导出暂由后续任务消费，在消费者接入前允许
//! dead_code / unused_imports，避免 clippy 误报。
#![allow(dead_code)]
#![allow(unused_imports)]

pub mod check;
pub mod download;
pub mod manifest;
pub mod version_policy;

pub use check::{UpdateInfo, UpdateKind};
pub use download::DownloadProgress;
