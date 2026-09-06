//! 更新管线的共享状态（Tauri manage）——后端权威执行状态
//!
//! 与旧 `updater/state.rs` 的区别：会话从"内部凭据"升级为显式阶段机。
//! 对外只投影简单快照（[`UpdateState`]，含阶段 + 展示数据），下载凭据
//! （`artifact` / 产物路径等）留在会话内部，不暴露给前端。
//!
//! # 并发模型
//!
//! - `session`：单槽会话，所有读写经 [`UpdaterState::mutate`] 短临界区完成
//!   （跨 await 不持锁）；
//! - 防重入由阶段推导（`Checking` / `Downloading` 即忙碌），不需要独立 busy 标志；
//! - `cancel`：下载取消信号，download 每 chunk 检查，结束后由编排层复位。

use crate::common::entity::update::{UpdateSession, UpdateState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// 更新管线的共享状态（通过 `tauri::Builder::manage` 注入）
#[derive(Default)]
pub struct UpdaterState {
    /// 当前更新会话（含阶段 + 展示信息 + 下载凭据）
    session: Mutex<UpdateSession>,
    /// 下载取消请求：前端 `cancel` 置位，download 每 chunk 检查，结束后复位
    cancel: AtomicBool,
}

impl UpdaterState {
    /// 在短临界区内读写会话并返回投影快照
    ///
    /// 闭包返回 `Err` 时回滚本次修改（用于入口防重入等校验，状态不落变更）。
    /// 调用方不得在闭包内执行任何 `.await`（锁不跨 await）。
    pub fn mutate(
        &self,
        f: impl FnOnce(&mut UpdateSession) -> Result<(), String>,
    ) -> Result<UpdateState, String> {
        let mut guard = self.session.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut guard)?;
        Ok(guard.snapshot())
    }

    /// 只读投影当前快照（供 `state` 查询命令）
    pub fn snapshot(&self) -> UpdateState {
        self.session
            .lock()
            .map(|g| g.snapshot())
            .unwrap_or_else(|e| e.into_inner().snapshot())
    }

    /// 克隆当前会话（供编排层在锁外读取凭据与产物路径）
    pub fn session(&self) -> UpdateSession {
        self.session
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
    }

    /// 下载取消标志的共享引用（download 每 chunk 检查）
    pub fn is_cancelled(&self) -> &AtomicBool {
        &self.cancel
    }

    /// 请求取消当前下载（前端 `cancel` 调用；仅置位）
    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// 复位取消标志（download 结束后由编排层调用，成功 / 失败 / 取消均复位）
    pub fn reset_cancel(&self) {
        self.cancel.store(false, Ordering::Relaxed);
    }
}
