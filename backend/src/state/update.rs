//! 更新管线的共享状态（Tauri manage）——后端权威执行状态
//!
//! 会话是**内部**状态（不 Serialize、不导出），持有下载凭据等跨命令事实；
//! 对外只经 [`UpdateSession::view`] 投影为裁剪的展示视图 [`UpdateView`]，
//! 命令返回值与广播事件共用该视图，前端 store 用单一 apply 逻辑消费。
//!
//! # 并发模型
//!
//! - `session`：单槽会话，所有读写经 [`UpdaterState::mutate`] 短临界区完成
//!   （跨 await 不持锁）；
//! - 防重入由阶段推导（`Checking` / `Downloading` 即忙碌），不需要独立 busy 标志；
//! - `cancel`：下载取消信号，download 每 chunk 检查，结束后由编排层复位。

use crate::common::entity::update::{UpdateSession, UpdateView};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// 更新管线的共享状态（通过 `tauri::Builder::manage` 注入）
#[derive(Default)]
pub struct UpdaterState {
    /// 当前更新会话（含阶段 + 展示信息 + 下载凭据，内部使用）
    session: Mutex<UpdateSession>,
    /// 下载取消请求：前端 `cancel` 置位，download 每 chunk 检查，结束后复位
    cancel: AtomicBool,
}

impl UpdaterState {
    /// 在短临界区内读写会话并返回投影展示视图
    ///
    /// 闭包返回 `Err` 时回滚本次修改（用于入口防重入等校验，状态不落变更）。
    /// 调用方不得在闭包内执行任何 `.await`（锁不跨 await）。
    pub fn mutate(
        &self,
        f: impl FnOnce(&mut UpdateSession) -> Result<(), String>,
    ) -> Result<UpdateView, String> {
        let mut guard = self.session.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut guard)?;
        Ok(guard.view())
    }

    /// 只读投影当前展示视图（供 `state` 查询命令与广播）
    pub fn view(&self) -> UpdateView {
        self.session
            .lock()
            .map(|g| g.view())
            .unwrap_or_else(|e| e.into_inner().view())
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
