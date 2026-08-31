//! 统一更新管线（自研，替代前端 tauri-plugin-updater 驱动）
//!
//! 任务 01：manifest 解析与版本策略。
//! 本模块为纯数据层，不涉及网络、不涉及安装。
//!
//! 数据层类型暂由后续任务（download/verify/install 等）消费，
//! 在消费者接入前允许 dead_code，避免 clippy 误报。
#![allow(dead_code)]

pub mod manifest;
pub mod version_policy;
