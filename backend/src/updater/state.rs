//! 更新管线的共享状态（Tauri manage）与防重入守卫
//!
//! 选型说明：`check_update` 的结果存于 [`UpdaterState::last_check`]
//! （`Mutex<Option<UpdateInfo>>`），避免前端跨命令传复杂对象——与插件用
//! resource 表存 `Update` 的语义等价，但更简单：本管线只需"最近一次检查结果"，
//! 无需管理跨窗口/多实例的资源生命周期。
//!
//! `busy` 标志配合 [`BusyGuard`] 实现 check / download 期间防重入：命令入口
//! 先 `try_enter`，拿不到执行权说明已有更新操作在进行，直接拒绝重复触发。

use crate::updater::check::UpdateInfo;
use std::sync::Mutex;

/// 更新管线的共享状态（通过 `tauri::Builder::manage` 注入）
pub struct UpdaterState {
    /// 最近一次 check 的 UpdateInfo（`download_and_install_update` 消费）
    last_check: Mutex<Option<UpdateInfo>>,
    /// 防重入：check / download_and_install 进行中为 `true`
    busy: Mutex<bool>,
}

impl Default for UpdaterState {
    fn default() -> Self {
        Self {
            last_check: Mutex::new(None),
            busy: Mutex::new(false),
        }
    }
}

impl UpdaterState {
    /// 防重入：尝试获取更新操作执行权
    ///
    /// 已有 check / download_and_install 在进行中时返回错误（前端提示"稍候"）。
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

    /// 读取最近一次检查结果（克隆，避免跨 await 持有锁）
    pub fn last_check(&self) -> Option<UpdateInfo> {
        self.last_check
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
    }

    /// 写入最近一次检查结果
    pub fn set_last_check(&self, info: Option<UpdateInfo>) -> Result<(), String> {
        let mut slot = self.last_check.lock().unwrap_or_else(|e| e.into_inner());
        *slot = info;
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
