//! 更新管线的共享状态（Tauri manage）——后端权威执行状态。

use crate::updater::model::{DownloadProgress, UpdateError, UpdateSession, UpdateStatus, UpdateView};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// 一次下载会话的瞬时资源（原子槽，无锁）
///
/// 热路径（每 chunk）只碰原子，不碰 session 锁。`total` 以 `0` 哨兵表达"未知"。
#[derive(Default)]
pub struct DownloadSlot {
    downloaded: AtomicU64,
    total: AtomicU64,
    cancel: AtomicBool,
}

impl DownloadSlot {
    pub fn set_progress(&self, p: DownloadProgress) {
        self.downloaded.store(p.downloaded, Ordering::Relaxed);
        self.total.store(p.total.unwrap_or(0), Ordering::Relaxed);
    }

    /// 读进度快照（`0` 哨兵还原 `None`）
    pub fn snapshot(&self) -> DownloadProgress {
        DownloadProgress {
            downloaded: self.downloaded.load(Ordering::Relaxed),
            total: match self.total.load(Ordering::Relaxed) {
                0 => None,
                n => Some(n),
            },
        }
    }

    /// 下载取消标志的共享引用（download 每 chunk 检查）
    pub fn cancel_flag(&self) -> &AtomicBool {
        &self.cancel
    }

    /// 请求取消当前下载（仅置位）
    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// 下载结束复位：清进度与取消位（成功 / 失败 / 取消都要调用）
    pub fn finish(&self) {
        self.downloaded.store(0, Ordering::Relaxed);
        self.total.store(0, Ordering::Relaxed);
        self.cancel.store(false, Ordering::Relaxed);
    }
}

/// 更新管线的共享状态（通过 `tauri::Builder::manage` 注入）
#[derive(Default)]
pub struct UpdaterState {
    session: Mutex<UpdateSession>,
    slot: DownloadSlot,
}

impl UpdaterState {
    /// 在短临界区内读写会话并返回投影视图
    ///
    /// 闭包返回 `Err` 时回滚本次修改。**闭包内禁止 `.await`**（锁不跨 await）。
    pub fn mutate(&self, f: impl FnOnce(&mut UpdateSession) -> Result<(), String>) -> Result<UpdateView, String> {
        let mut guard = self.session.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut guard)?;
        Ok(guard.view())
    }

    /// 只读投影当前视图
    pub fn view(&self) -> UpdateView {
        self.session
            .lock()
            .map(|g| g.view())
            .unwrap_or_else(|e| e.into_inner().view())
    }

    /// 克隆当前会话（供编排层在锁外读取凭据）
    pub fn session(&self) -> UpdateSession {
        self.session
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
    }

    /// 落错并返回视图（`status = Error` + `error` 同锁写入，其余字段保留）
    pub fn fail(&self, error: UpdateError) -> Result<UpdateView, String> {
        self.mutate(|s| {
            s.status = UpdateStatus::Error;
            s.error = Some(error);
            Ok(())
        })
    }

    /// 迁移到无错误的阶段并返回视图
    pub fn enter(&self, status: UpdateStatus) -> Result<UpdateView, String> {
        self.mutate(|s| {
            s.status = status;
            s.error = None;
            Ok(())
        })
    }

    /// 当前下载会话的瞬时资源
    pub fn slot(&self) -> &DownloadSlot {
        &self.slot
    }
}
