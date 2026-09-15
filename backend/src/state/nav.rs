//! 跨窗口导航的待取载荷（Tauri manage）——**唯一**的跨窗口数据通路
//!
//! # 为什么是"单槽覆盖"而不是队列
//!
//! 投递的语义是"把那个窗口带到某个样子"，是**状态**不是**消息**。所以最新的一次覆盖
//! 前一次：连点两个链接不会排队弹两次。这与更新器 view 通道同源——状态只从命令取，
//! 事件只做提示。
//!
//! # 为什么必须"存着等取"
//!
//! 目标窗口可能是**懒创建**的，建窗那一刻目标前端还没开始监听，任何广播都必丢。
//! 载荷先落在后端，等前端自己来取（`nav_take_pending`），时机问题就被抹平了。

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

/// 一次待投递的导航
///
/// `route` 与 `payload` 对后端**全程不透明**：后端只切 `route` 的第一段当窗口名用
/// （"域即窗口名"这条隐含规则），此外不解释任何内容，也不需要镜像一套前端类型。
#[derive(Clone, Debug, Serialize)]
pub struct PendingNav {
    /// 完整地址（`#/<窗口名>/<段...>`），后端只读第一段
    pub route: String,
    /// 载荷：原样搬运，不校验、不转换（`null` / 缺省即"只有身份没有数据"）
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
