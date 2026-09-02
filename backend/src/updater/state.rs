//! 更新管线的共享状态（Tauri manage）与防重入守卫
//!
//! 单槽位 [`UpdaterState::pending`] 承载**一次更新会话**（跨命令的唯一凭据）：
//! check 批准后写入（尚未下载），download 成功后补上落盘路径，install 消费落盘
//! 产物——"下载完成、待重启安装"这一阶段由 [`PendingUpdate::downloaded_path`]
//! 的 `Some` 表达，前端 `Downloaded` 状态只是它的镜像。
//!
//! 选型说明：命令契约为两段式（check → download → install），前端不持句柄，
//! 单窗口、单更新流，单槽位即最自洽的"后端统一源"（等价于插件 rid-resource
//! 的多实例方案在单实例场景下的退化形态，省去句柄与生命周期管理）。
//!
//! `busy` 标志配合 [`BusyGuard`] 实现 check / download / install 期间防重入：
//! 命令入口先 `try_enter`，拿不到执行权说明已有更新操作在进行，直接拒绝重复触发。

use crate::updater::check::{UpdateInfo, UpdateKind};
use crate::updater::manifest::{Artifact, Severity};
use semver::Version;
use std::path::PathBuf;
use std::sync::Mutex;

/// 一次更新会话（跨 check → download → install 共享的唯一凭据）
///
/// 内容限定为"各步共享、一变就乱套的不变事实"：判定基线（current_version）、
/// 批准下载的产物（artifact）与安装形态（kind/severity/force）。notes/date/raw_json
/// 等纯展示字段不进会话（前端由 check 返回值直接持有）。
#[derive(Debug, Clone)]
pub struct PendingUpdate {
    /// 目标版本
    pub version: Version,
    /// 安装形态（决定 install 走 NSIS 还是 Portable 安装器）
    pub kind: UpdateKind,
    /// 严重程度（download 复核 / force 语义依赖）
    pub severity: Severity,
    /// 是否强制
    pub force: bool,
    /// 批准下载的产物（download 消费）
    pub artifact: Artifact,
    /// 判定时的基线版本（download 入口复核对照；应用当前运行版本）
    pub current_version: Version,
    /// `None` = 已批准未下载；`Some` = 已下载待安装（install 消费的落盘产物）
    pub downloaded_path: Option<PathBuf>,
}

impl PendingUpdate {
    /// 由 check 返回值构造会话（下载前的初始形态）
    pub fn from_update(info: &UpdateInfo, current_version: Version) -> Self {
        Self {
            version: info.version.clone(),
            kind: info.kind,
            severity: info.severity,
            force: info.force,
            artifact: info.artifact.clone(),
            current_version,
            downloaded_path: None,
        }
    }
}

/// 更新管线的共享状态（通过 `tauri::Builder::manage` 注入）
pub struct UpdaterState {
    /// 当前更新会话（`None` = 无待办更新/已作废）
    pending: Mutex<Option<PendingUpdate>>,
    /// 防重入：check / download / install 进行中为 `true`
    busy: Mutex<bool>,
}

impl Default for UpdaterState {
    fn default() -> Self {
        Self {
            pending: Mutex::new(None),
            busy: Mutex::new(false),
        }
    }
}

impl UpdaterState {
    /// 防重入：尝试获取更新操作执行权
    ///
    /// 已有 check / download / install 在进行中时返回错误（前端提示"稍候"）。
    /// 返回的 [`BusyGuard`] 在 Drop 时自动释放 busy 标志，覆盖成功/失败/panic
    /// 所有路径；成功安装路径进程直接 `exit(0)`，guard 的 Drop 不再需要。
    pub fn try_enter(state: &UpdaterState) -> Result<BusyGuard<'_>, String> {
        let mut busy = state.busy.lock().unwrap_or_else(|e| e.into_inner());
        if *busy {
            return Err("更新操作正在进行中，请稍候再试".to_string());
        }
        *busy = true;
        Ok(BusyGuard { state, active: true })
    }

    /// 读取当前更新会话（克隆，避免跨 await 持有锁）
    pub fn pending(&self) -> Option<PendingUpdate> {
        self.pending
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
    }

    /// 写入/清空更新会话（`None` = 作废）
    pub fn set_pending(&self, session: Option<PendingUpdate>) -> Result<(), String> {
        let mut slot = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        *slot = session;
        Ok(())
    }
}

/// 防重入守卫：Drop 时恢复 busy 标志
///
/// 只持有 `&UpdaterState`（而非 MutexGuard），可安全跨 `.await` 存活。
pub struct BusyGuard<'a> {
    state: &'a UpdaterState,
    active: bool,
}

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        if self.active {
            if let Ok(mut busy) = self.state.busy.lock() {
                *busy = false;
            }
        }
    }
}
