//! 跨窗口导航的待取载荷

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

/// 一次待投递的导航
#[derive(Clone, Debug, Serialize)]
pub struct PendingNav {
    pub route: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

/// 每个窗口一个单槽的待取载荷
#[derive(Default)]
pub struct NavState {
    pending: Mutex<HashMap<String, PendingNav>>,
}

impl NavState {
    /// 投递（覆盖式，last-wins）
    pub fn put(&self, host: &str, nav: PendingNav) {
        self.guard().insert(host.to_string(), nav);
    }

    /// 取走**并清空**：不清空的话，下次唤醒会把同一个旧地址再应用一次
    pub fn take(&self, host: &str) -> Option<PendingNav> {
        self.guard().remove(host)
    }

    /// 窗口销毁时清理：否则窗口重建后的首帧会应用一个早就过期的导航
    pub fn clear(&self, host: &str) {
        self.guard().remove(host);
    }

    fn guard(&self) -> MutexGuard<'_, HashMap<String, PendingNav>> {
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }
}
