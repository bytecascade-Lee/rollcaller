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
//! - `slot`（[`DownloadSlot`]）：一次下载会话的瞬时资源——取消标志 + 进度原子，
//!   下载的每 chunk 只写原子（不碰 session 锁）；进度是瞬时数据、只服务 download
//!   通道窄帧，不进入 session / 对外视图。

use crate::common::entity::update::{DownloadProgress, UpdateSession, UpdateView};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// 一次下载会话的瞬时资源（原子槽，无锁）
///
/// 生命周期与 `Downloading` 期绑定：进入下载时启用、结束后由编排层复位进度
/// （`cancel` 单独复位）。取消与进度同属"一次下载会话"，故合为一槽；
/// 热路径（每 chunk）只碰原子、不碰 session 锁。
///
/// `total` 以 `0` 哨兵表达"未知"（下载大小不可能为 0）；[`DownloadSlot::snapshot`]
/// 读出时还原为 `None`。
#[derive(Default)]
pub struct DownloadSlot {
    /// 已下载字节数
    downloaded: AtomicU64,
    /// 总字节数（`0` = 未知）
    total: AtomicU64,
    /// 下载取消请求（download 每 chunk 检查）
    cancel: AtomicBool,
}

impl DownloadSlot {
    /// 写进度（每 chunk 仅 store 原子，不碰 session 锁）
    pub fn set_progress(&self, p: DownloadProgress) {
        self.downloaded.store(p.downloaded, Ordering::Relaxed);
        self.total.store(p.total.unwrap_or(0), Ordering::Relaxed);
    }

    /// 读进度快照（`0` 哨兵还原 `None`）——download 通道窄帧的节流读取点
    pub fn snapshot(&self) -> DownloadProgress {
        DownloadProgress {
            downloaded: self.downloaded.load(Ordering::Relaxed),
            total: match self.total.load(Ordering::Relaxed) {
                0 => None,
                n => Some(n),
            },
        }
    }

    /// 复位进度（下载结束调用；`cancel` 另由 [`DownloadSlot::reset_cancel`] 复位）
    pub fn reset(&self) {
        self.downloaded.store(0, Ordering::Relaxed);
        self.total.store(0, Ordering::Relaxed);
    }

    /// 下载取消标志的共享引用（download 每 chunk 检查）
    pub fn cancel_flag(&self) -> &AtomicBool {
        &self.cancel
    }

    /// 请求取消当前下载（仅置位；download 感知后清理 `.part` 返回）
    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// 复位取消标志（download 结束后由编排层调用，成功 / 失败 / 取消均复位）
    pub fn reset_cancel(&self) {
        self.cancel.store(false, Ordering::Relaxed);
    }
}

/// 更新管线的共享状态（通过 `tauri::Builder::manage` 注入）
#[derive(Default)]
pub struct UpdaterState {
    /// 当前更新会话（含阶段 + 展示信息 + 下载凭据，内部使用）
    session: Mutex<UpdateSession>,
    /// 当前下载会话的瞬时资源（取消 + 进度原子，编排层独占读写）
    slot: DownloadSlot,
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

    /// 当前下载会话的瞬时资源（供编排层写进度 / 节流快照 / 复位 / 取取消标志）
    pub fn slot(&self) -> &DownloadSlot {
        &self.slot
    }

    /// 下载取消标志的共享引用（download 每 chunk 检查）
    pub fn is_cancelled(&self) -> &AtomicBool {
        self.slot.cancel_flag()
    }

    /// 请求取消当前下载（前端 `cancel` 调用；仅置位）
    pub fn request_cancel(&self) {
        self.slot.request_cancel();
    }

    /// 复位取消标志（download 结束后由编排层调用，成功 / 失败 / 取消均复位）
    pub fn reset_cancel(&self) {
        self.slot.reset_cancel();
    }
}
