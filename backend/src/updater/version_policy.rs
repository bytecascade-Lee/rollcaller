//! 版本更新决策
//!
//! 策略语义为「最小可接受更新单元」（floor），而非依赖库场景的「最大容忍变化量」（cap）：
//! 桌面软件没有 API 兼容包袱，用户关心的不是"变化太大"，而是"更新太频繁、每次都要确认/重启太烦"。
//! 因此用户设定的等级表示：至少要有这么大的变化才值得更新，
//! 低于门槛的更新被忽略（延迟聚合，等下一个达标的版本一次跳过去）。
//!
//! 分发平台属于获取层（Fetcher Layer）的职责，在调用 [`decide`] 之前就已经通过平台拉取到了 `latest` 版本。

use semver::Version;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use ts_rs::TS;

/// 更新决策结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, TS)]
pub enum UpdateDecision {
    /// 不更新
    Skip,
    /// 更新
    Update,
}

/// 幅度门槛（floor）：最少需要多大的数字变化才触发更新。
///
/// 由 `Option<UpdateLevel>` 承载：`None` 表示禁止所有更新。
///
/// | 门槛 | 1.2.1→1.2.2 | 1.2.1→1.3.0 | 1.2.1→2.0.0 |
/// |---|---|---|---|
/// | Patch | 更新 | 更新 | 更新 |
/// | Minor | 跳过 | 更新 | 更新 |
/// | Major | 跳过 | 跳过 | 更新 |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
pub enum UpdateLevel {
    /// 补丁及以上
    Patch,
    /// 次版本及以上
    Minor,
    /// 仅主版本
    Major,
}

/// 发布阶段（Release Channel）：决定版本列表的可见性。
///
/// - **Stable**：只接收稳定版。latest 为预发布版本时一律跳过（升级、降级同判）。
/// - **Prerelease**：接收预发布版本，且预发布相关路径有两条特殊规则：
///   - 「预发布 → 预发布」是**通道内递进**（如 1.2.0-rc.1 → 1.2.0-rc.2）：进入通道时
///     风险已被接受，跟随通道、不再询问，**不适用幅度门槛**；
///   - 「预发布 → 稳定版」是**逃逸通道**（如 1.2.0-rc.1 → 1.2.0）：始终更新，
///     不适用幅度门槛与通道，否则用户会永远卡在预发布版本上。
///
/// 而「稳定版 → 预发布」是**通道切换**（如 1.2.0 → 1.3.0-rc.1）：需要通道为 Prerelease
/// 才可见，且仍按幅度门槛判断（数字层面的补丁/次版本/主版本变化）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
pub enum UpdateChannel {
    /// 正式版
    Stable,
    /// 预发布版
    Prerelease,
}

impl Default for UpdateChannel {
    fn default() -> Self {
        Self::Stable
    }
}

/// 出厂推荐默认策略：`level = Minor` + `channel = Stable`。
pub const DEFAULT_LEVEL: UpdateLevel = UpdateLevel::Minor;
pub const DEFAULT_CHANNEL: UpdateChannel = UpdateChannel::Stable;

/// 设置加载/保存时的合法性校验。
///
/// - 若 `level` 为 `None`（禁止更新），则 `channel` 无任何作用，统一修正为 `Stable`，防止脏数据进入逻辑判定；
///   存储层允许该组合存在，只是判定时直接跳过，前端无需做复杂的互斥校验。
/// - 其余组合原样返回。
pub fn validate(level: Option<UpdateLevel>, channel: UpdateChannel) -> (Option<UpdateLevel>, UpdateChannel) {
    if level.is_none() {
        (None, UpdateChannel::Stable)
    } else {
        (level, channel)
    }
}

/// 变化幅度：升级路径中只比较主/次/补丁号（预发布标识不参与幅度分类）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VersionChange {
    /// 主版本号
    Major,
    /// 次版本号
    Minor,
    /// 补丁号
    Patch,
}

/// 版本更新决策入口。
///
/// # 参数：
/// - `current` / `latest`：当前版本与清单中的目标版本
/// - `level`：幅度门槛（`None` = 禁止所有更新，原 `Never` 行为）
/// - `channel`：发布阶段（Stable / Prerelease），只负责"预发布版本是否可见"的门禁
/// - `allow_downgrade`：是否允许降级（与幅度门槛/通道正交，显式允许才放行）
///
/// # 决策流程：
/// 1. `level` 为 `None` → `Skip`；
/// 2. 稳定通道下 `latest` 为预发布 → `Skip`（升级、降级同判）；
/// 3. `latest < current`（降级）→ 仅当 `allow_downgrade` 为真才 `Update`；
/// 4. `current` 为预发布、`latest` 为稳定版（逃逸通道）→ `Update`；
/// 5. `current`、`latest` 均为预发布（通道内递进）→ `Update`；
/// 6. 其余（稳定→稳定，或稳定→预发布切换）→ 按 `level` 阈值判断数字变化幅度。
pub fn decide(
    current: &Version,
    latest: &Version,
    level: Option<UpdateLevel>,
    channel: UpdateChannel,
    allow_downgrade: bool,
) -> UpdateDecision {
    // 1. level 为 None：无条件跳过
    let Some(level) = level else {
        return UpdateDecision::Skip;
    };

    // 2. 通道门禁：稳定通道下 latest 为预发布版本 → 一律跳过（升级/降级同判，保证稳定通道用户不会接触到任何预发布版本）
    if channel == UpdateChannel::Stable && !latest.pre.is_empty() {
        return UpdateDecision::Skip;
    }

    // 3. 版本比较
    match latest.cmp(current) {
        Ordering::Equal => UpdateDecision::Skip,
        Ordering::Less => {
            // 降级：仅当显式允许时更新（不受幅度门槛影响；通道门禁已在第 2 步处理）
            if allow_downgrade {
                UpdateDecision::Update
            } else {
                UpdateDecision::Skip
            }
        }
        Ordering::Greater => decide_upgrade(current, latest, level),
    }
}

/// 升级路径的决策：处理预发布的特殊形态（逃逸 / 递进）后，按幅度门槛（floor）判断
fn decide_upgrade(current: &Version, latest: &Version, level: UpdateLevel) -> UpdateDecision {
    let current_is_pre = !current.pre.is_empty();
    let latest_is_pre = !latest.pre.is_empty();

    // 4. 逃逸通道：预发布 → 稳定版，始终更新（不适用幅度门槛，否则用户永远卡在预发布）
    if current_is_pre && !latest_is_pre {
        return UpdateDecision::Update;
    }
    // 5. 通道内递进：预发布 → 预发布，跟随通道（不适用幅度门槛，风险已在进入通道时接受；
    //    通道门禁已在上游确保当前通道允许预发布）
    if current_is_pre && latest_is_pre {
        return UpdateDecision::Update;
    }
    // 6. 其余：稳定 → 稳定，或稳定 → 预发布的通道切换 → 按幅度门槛判断
    let change = classify_numeric_change(current, latest);
    if floor_satisfied(change, level) {
        UpdateDecision::Update
    } else {
        UpdateDecision::Skip
    }
}

/// 升级路径中的幅度分类：只比较主/次/补丁号。
///
/// 升级路径上数字必然不降；主/次/补丁号完全相同（仅预发布标识变化）的情况已被
/// decide_upgrade 的通道内递进分支接管，此处回退为 Patch 不会被执行到。
fn classify_numeric_change(current: &Version, latest: &Version) -> VersionChange {
    if latest.major > current.major {
        VersionChange::Major
    } else if latest.minor > current.minor {
        VersionChange::Minor
    } else {
        VersionChange::Patch
    }
}

/// floor 语义的允许表：变化幅度 ≥ 门槛则允许
fn floor_satisfied(change: VersionChange, floor: UpdateLevel) -> bool {
    use VersionChange::*;
    match floor {
        UpdateLevel::Patch => matches!(change, Patch | Minor | Major),
        UpdateLevel::Minor => matches!(change, Minor | Major),
        UpdateLevel::Major => matches!(change, Major),
    }
}
