//! 版本更新决策
//!
//! 策略语义为「最小可接受更新单元」（floor），而非依赖库场景的「最大容忍变化量」（cap）：
//! 桌面软件没有 API 兼容包袱，用户关心的不是"变化太大"，而是"更新太频繁、每次都要确认/重启太烦"。
//! 因此用户设定的等级表示：至少要有这么大的变化才值得更新，
//! 低于门槛的更新被忽略（延迟聚合，等下一个达标的版本一次跳过去）。

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

/// 当前默认策略：`level = Patch` + `channel = Stable`。
pub const DEFAULT_LEVEL: UpdateLevel = UpdateLevel::Patch;
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

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version;
    use UpdateChannel::*;
    use UpdateLevel::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    /// 便捷断言：失败时打印全部参数，便于定位
    fn assert_decide(
        current: &str,
        latest: &str,
        level: Option<UpdateLevel>,
        channel: UpdateChannel,
        allow_downgrade: bool,
        expected: UpdateDecision,
    ) {
        assert_eq!(
            decide(&v(current), &v(latest), level, channel, allow_downgrade),
            expected,
            "decide(current={current}, latest={latest}, level={level:?}, channel={channel:?}, allow_downgrade={allow_downgrade})"
        );
    }

    /// level 为 None 时无条件跳过：升级 / 降级 / 同版本 / 预发布，任何通道与降级开关都不放行
    #[test]
    fn level_none_disables_all_updates() {
        for channel in [Stable, Prerelease] {
            for downgrade in [false, true] {
                // 升级主版本
                assert_decide("1.2.1", "2.0.0", None, channel, downgrade, UpdateDecision::Skip);
                // 升级次版本
                assert_decide("1.2.1", "1.3.0", None, channel, downgrade, UpdateDecision::Skip);
                // 升级补丁
                assert_decide("1.2.1", "1.2.2", None, channel, downgrade, UpdateDecision::Skip);
                // 降级
                assert_decide("1.3.0", "1.2.0", None, channel, downgrade, UpdateDecision::Skip);
                // 同版本
                assert_decide("1.2.3", "1.2.3", None, channel, downgrade, UpdateDecision::Skip);
                // 预发布通道切换
                assert_decide("1.2.0", "1.3.0-rc.1", None, channel, downgrade, UpdateDecision::Skip);
                // 预发布通道逃逸
                assert_decide("1.2.0-rc.1", "1.2.0", None, channel, downgrade, UpdateDecision::Skip);
                // 预发布通道递进
                assert_decide("1.2.0-rc.1", "1.2.0-rc.2", None, channel, downgrade, UpdateDecision::Skip);
            }
        }
    }

    /// 稳定通道下 latest 为预发布 → 一律跳过：升级 / 降级 / 同号稳定→预发布，与 level、降级开关无关
    #[test]
    fn stable_channel_blocks_prerelease() {
        for level in [Some(Patch), Some(Minor), Some(Major)] {
            for downgrade in [false, true] {
                // 升级：稳定 → 预发布（通道切换被拦）
                assert_decide("1.2.1", "1.3.0-rc.1", level, Stable, downgrade, UpdateDecision::Skip);
                // 预发布 → 预发布（递进也被拦：latest 是预发布即不可见）
                assert_decide("1.2.0-rc.1", "1.2.0-rc.2", level, Stable, downgrade, UpdateDecision::Skip);
                // 降级：预发布 → 预发布（门禁先于降级分支，显式允许降级也不放行）
                assert_decide("1.3.0-rc.1", "1.2.0-rc.1", level, Stable, downgrade, UpdateDecision::Skip);
                // 同号稳定 → 预发布（semver 中 1.2.0-rc.1 < 1.2.0，属降级路径，门禁同样先拦）
                assert_decide("1.2.0", "1.2.0-rc.1", level, Stable, downgrade, UpdateDecision::Skip);
            }
        }
    }

    /// 版本完全相同：无论 level / channel / 降级开关，一律跳过
    #[test]
    fn equal_versions_always_skip() {
        for level in [None, Some(Patch), Some(Minor), Some(Major)] {
            for channel in [Stable, Prerelease] {
                assert_decide("1.2.3", "1.2.3", level, channel, false, UpdateDecision::Skip);
                assert_decide("1.2.3", "1.2.3", level, channel, true, UpdateDecision::Skip);
                assert_decide("1.2.3-alpha", "1.2.3-alpha", level, channel, false, UpdateDecision::Skip);
            }
        }
    }

    /// 降级仅受 allow_downgrade 约束：不受 level 门槛影响，但受稳定通道门禁约束
    #[test]
    fn downgrade_respects_flag() {
        // latest 为稳定版（不触发门禁）：level 任意，仅由 allow_downgrade 决定
        for level in [Some(Patch), Some(Minor), Some(Major)] {
            assert_decide("1.3.0", "1.2.0", level, Stable, false, UpdateDecision::Skip);
            assert_decide("1.3.0", "1.2.0", level, Stable, true, UpdateDecision::Update);
            assert_decide("1.3.0", "1.2.0", level, Prerelease, false, UpdateDecision::Skip);
            assert_decide("1.3.0", "1.2.0", level, Prerelease, true, UpdateDecision::Update);
        }
        // 预发布通道下的降级同样只认 allow_downgrade
        assert_decide(
            "1.3.0-rc.1",
            "1.2.0-rc.1",
            Some(Major),
            Prerelease,
            false,
            UpdateDecision::Skip,
        );
        assert_decide(
            "1.3.0-rc.1",
            "1.2.0-rc.1",
            Some(Major),
            Prerelease,
            true,
            UpdateDecision::Update,
        );
    }

    /// 预发布 → 稳定版：始终更新，不适用幅度门槛（与 level 无关）
    #[test]
    fn escape_prerelease_to_stable_always_updates() {
        for level in [Some(Patch), Some(Minor), Some(Major)] {
            for channel in [Prerelease, Stable] {
                // 同版本逃逸
                assert_decide("1.2.0-rc.1", "1.2.0", level, channel, false, UpdateDecision::Update);
                // 跨补丁逃逸
                assert_decide("1.2.0-rc.1", "1.2.1", level, channel, false, UpdateDecision::Update);
                // 跨次版本逃逸
                assert_decide("1.2.0-rc.1", "1.3.0", level, channel, false, UpdateDecision::Update);
                // 跨主版本逃逸
                assert_decide("1.2.0-rc.1", "2.0.0", level, channel, false, UpdateDecision::Update);
            }
        }
    }

    /// 预发布 → 预发布：跟随通道无条件更新，不适用幅度门槛（与 level 无关）
    #[test]
    fn prerelease_progression_skips_floor() {
        for level in [Some(Patch), Some(Minor), Some(Major)] {
            // 同号递进（rc.1 → rc.2）
            assert_decide("1.2.0-rc.1", "1.2.0-rc.2", level, Prerelease, false, UpdateDecision::Update);
            // 跨补丁递进
            assert_decide("1.2.0-rc.1", "1.2.1-rc.1", level, Prerelease, false, UpdateDecision::Update);
            // 跨次版本递进
            assert_decide("1.2.0-rc.1", "1.3.0-rc.1", level, Prerelease, false, UpdateDecision::Update);
            // 数字标识排序：alpha.2 → alpha.10 是升级（10 > 2，数字而非字典序）
            assert_decide(
                "1.2.0-alpha.2",
                "1.2.0-alpha.10",
                level,
                Prerelease,
                false,
                UpdateDecision::Update,
            );
        }
    }

    /// floor 全量表：3 种门槛 × 3 种数字变化（稳定 → 稳定）
    #[test]
    fn floor_matrix_stable_to_stable() {
        // Patch 门槛：任何升级都更新
        assert_decide("1.2.1", "1.2.2", Some(Patch), Stable, false, UpdateDecision::Update);
        assert_decide("1.2.1", "1.3.0", Some(Patch), Stable, false, UpdateDecision::Update);
        assert_decide("1.2.1", "2.0.0", Some(Patch), Stable, false, UpdateDecision::Update);
        // Minor 门槛：忽略补丁
        assert_decide("1.2.1", "1.2.2", Some(Minor), Stable, false, UpdateDecision::Skip);
        assert_decide("1.2.1", "1.3.0", Some(Minor), Stable, false, UpdateDecision::Update);
        assert_decide("1.2.1", "2.0.0", Some(Minor), Stable, false, UpdateDecision::Update);
        // Major 门槛：仅主版本
        assert_decide("1.2.1", "1.2.2", Some(Major), Stable, false, UpdateDecision::Skip);
        assert_decide("1.2.1", "1.3.0", Some(Major), Stable, false, UpdateDecision::Skip);
        assert_decide("1.2.1", "2.0.0", Some(Major), Stable, false, UpdateDecision::Update);
    }

    /// 通道切换（稳定 → 预发布）：Prerelease 通道下仍按幅度门槛判断
    #[test]
    fn channel_switch_stable_to_prerelease_uses_floor() {
        // 补丁级切换（1.2.1 → 1.2.2-rc.1）
        assert_decide("1.2.1", "1.2.2-rc.1", Some(Patch), Prerelease, false, UpdateDecision::Update);
        assert_decide("1.2.1", "1.2.2-rc.1", Some(Minor), Prerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.1", "1.2.2-rc.1", Some(Major), Prerelease, false, UpdateDecision::Skip);
        // 次版本级切换（1.2.1 → 1.3.0-rc.1）
        assert_decide("1.2.1", "1.3.0-rc.1", Some(Patch), Prerelease, false, UpdateDecision::Update);
        assert_decide("1.2.1", "1.3.0-rc.1", Some(Minor), Prerelease, false, UpdateDecision::Update);
        assert_decide("1.2.1", "1.3.0-rc.1", Some(Major), Prerelease, false, UpdateDecision::Skip);
        // 主版本级切换（1.2.1 → 2.0.0-rc.1）
        assert_decide("1.2.1", "2.0.0-rc.1", Some(Patch), Prerelease, false, UpdateDecision::Update);
        assert_decide("1.2.1", "2.0.0-rc.1", Some(Minor), Prerelease, false, UpdateDecision::Update);
        assert_decide("1.2.1", "2.0.0-rc.1", Some(Major), Prerelease, false, UpdateDecision::Update);
    }

    /// semver 排序边界：同号「稳定 → 预发布」（1.2.0 → 1.2.0-rc.1）是降级而非升级，
    /// Prerelease 通道下仅 allow_downgrade=true 才更新
    #[test]
    fn stable_to_same_number_prerelease_is_downgrade() {
        assert_decide("1.2.0", "1.2.0-rc.1", Some(Patch), Prerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.0", "1.2.0-rc.1", Some(Patch), Prerelease, true, UpdateDecision::Update);
    }

    /// semver crate 行为锁定：build metadata（+abc）是最后决胜键（规范偏离），
    /// 同号不同 build 判为 Greater → 幅度分类为 Patch → 按门槛判断
    #[test]
    fn build_metadata_counts_as_patch_change() {
        assert_decide("1.2.3+abc", "1.2.3+bcd", Some(Patch), Stable, false, UpdateDecision::Update);
        assert_decide("1.2.3+abc", "1.2.3+bcd", Some(Minor), Stable, false, UpdateDecision::Skip);
        // build metadata 不改变主/次/补丁幅度：1.2.3+abc → 1.2.4 仍是补丁升级
        assert_decide("1.2.3+abc", "1.2.4", Some(Patch), Stable, false, UpdateDecision::Update);
        assert_decide("1.2.3+abc", "1.2.4", Some(Minor), Stable, false, UpdateDecision::Skip);
    }

    /// validate：level 为 None 时 channel 一律修正为 Stable（含脏数据 None + Prerelease）
    #[test]
    fn validate_normalizes_none_channel() {
        assert_eq!(validate(None, Stable), (None, Stable));
        assert_eq!(validate(None, Prerelease), (None, Stable));
        // level 有值：原样返回
        assert_eq!(validate(Some(Patch), Stable), (Some(Patch), Stable));
        assert_eq!(validate(Some(Major), Prerelease), (Some(Major), Prerelease));
    }

    /// floor_satisfied 全量组合表：3 种门槛 × 3 种变化幅度
    #[test]
    fn floor_satisfied_table() {
        let cases: &[(VersionChange, UpdateLevel, bool)] = &[
            // Patch 门槛：任何升级都允许
            (VersionChange::Patch, Patch, true),
            (VersionChange::Minor, Patch, true),
            (VersionChange::Major, Patch, true),
            // Minor 门槛：忽略补丁
            (VersionChange::Patch, Minor, false),
            (VersionChange::Minor, Minor, true),
            (VersionChange::Major, Minor, true),
            // Major 门槛：仅主版本
            (VersionChange::Patch, Major, false),
            (VersionChange::Minor, Major, false),
            (VersionChange::Major, Major, true),
        ];
        for (change, floor, expected) in cases {
            assert_eq!(
                floor_satisfied(*change, *floor),
                *expected,
                "floor_satisfied(change={change:?}, floor={floor:?})"
            );
        }
    }

    /// classify_numeric_change 的三个分支 + 同数字回退 Patch
    #[test]
    fn classify_numeric_change_branches() {
        assert_eq!(classify_numeric_change(&v("1.2.1"), &v("2.0.0")), VersionChange::Major);
        assert_eq!(classify_numeric_change(&v("1.2.1"), &v("1.3.0")), VersionChange::Minor);
        assert_eq!(classify_numeric_change(&v("1.2.1"), &v("1.2.2")), VersionChange::Patch);
        // 主/次/补丁号完全相同（正常流程被递进分支接管，此处回退 Patch）
        assert_eq!(classify_numeric_change(&v("1.2.3"), &v("1.2.3-beta")), VersionChange::Patch);
    }

    /// 出厂默认值与文档推荐一致：Patch + Stable
    #[test]
    fn defaults_match_recommended_policy() {
        assert_eq!(DEFAULT_LEVEL, Patch);
        assert_eq!(DEFAULT_CHANNEL, Stable);
    }
}
