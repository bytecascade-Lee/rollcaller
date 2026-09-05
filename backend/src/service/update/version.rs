//! 版本更新决策
//!
//! 策略语义为「最小可接受更新单元」（floor），而非依赖库场景的「最大容忍变化量」（cap）：
//! 桌面软件没有 API 兼容包袱，用户关心的不是"变化太大"，而是"更新太频繁、每次都要确认/重启太烦"。
//! 因此用户设定的等级表示：至少要有这么大的变化才值得更新，
//! 低于门槛的更新被忽略（延迟聚合，等下一个达标的版本一次跳过去）。

use crate::common::constant::update::{DEFAULT_UPDATE_CHANNEL, DEFAULT_UPDATE_LEVEL};
use crate::common::enums::update::{UpdateChannel, UpdateDecision, UpdateLevel};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use ts_rs::TS;

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
/// - `level`：幅度门槛
/// - `channel`：发布阶段（Stable / Prerelease），只负责"预发布版本是否可见"的门禁
///
/// # 决策流程：
/// 1. `level` 为 `None` → `Skip`；
/// 2. 稳定通道下 `latest` 为预发布 → `Skip`；
/// 3. `latest < current`→ 仅当 `allow_downgrade` 为真才 `Update`；
/// 4. `current` 为预发布、`latest` 为稳定版（逃逸通道）→ `Update`；
/// 5. `current`、`latest` 均为预发布（通道内递进）→ `Update`；
/// 6. 其余（稳定→稳定，或稳定→预发布切换）→ 按 `level` 阈值判断数字变化幅度。
pub fn decide(current: &Version, latest: &Version, level: UpdateLevel, channel: UpdateChannel) -> UpdateDecision {
    // 1. level 为 None：无条件跳过
    if level == UpdateLevel::Never {
        return UpdateDecision::Skip;
    };

    // 2. 通道门禁：稳定通道下 latest 为预发布版本 → 一律跳过，保证稳定通道用户不会接触到任何预发布版本
    if channel == UpdateChannel::Stable && !latest.pre.is_empty() {
        return UpdateDecision::Skip;
    }

    // 3. 版本比较
    match latest.cmp(current) {
        Ordering::Equal => UpdateDecision::Skip,
        Ordering::Less => UpdateDecision::Skip,
        Ordering::Greater => decide_upgrade(current, latest, level),
    }
}

/// 升级路径的决策：处理预发布的特殊形态（逃逸 / 递进）后，按幅度门槛（floor）判断
fn decide_upgrade(current: &Version, latest: &Version, level: UpdateLevel) -> UpdateDecision {
    let current_is_pre = !current.pre.is_empty();
    let latest_is_pre = !latest.pre.is_empty();

    // 4. 逃逸通道：预发布 → 稳定版，始终更新
    if current_is_pre && !latest_is_pre {
        return UpdateDecision::Update;
    }
    // 5. 通道内递进：预发布 → 预发布，跟随通道（不适用幅度门槛，风险已在进入通道时接受；通道门禁已在上游确保当前通道允许预发布）
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
/// 升级路径上数字必然不降；主/次/补丁号完全相同（仅预发布标识变化）的情况已被 decide_upgrade 的通道内递进分支接管，此处回退为 Patch 不会被执行到。
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
        UpdateLevel::Never => false,
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
    fn assert_decide(current: &str, latest: &str, level: UpdateLevel, channel: UpdateChannel, expected: UpdateDecision) {
        assert_eq!(
            decide(&v(current), &v(latest), level, channel),
            expected,
            "decide(current={current}, latest={latest}, level={level:?}, channel={channel:?})"
        );
    }

    /// level 为 None 时无条件跳过：升级 / 同版本 / 预发布，任何通道都不放行
    #[test]
    fn level_none_disables_all_updates() {
        for channel in [Stable, Prerelease] {
            for downgrade in [false, true] {
                // 升级主版本
                assert_decide("1.2.1", "2.0.0", Never, channel, UpdateDecision::Skip);
                // 升级次版本
                assert_decide("1.2.1", "1.3.0", Never, channel, UpdateDecision::Skip);
                // 升级补丁
                assert_decide("1.2.1", "1.2.2", Never, channel, UpdateDecision::Skip);
                // 同版本
                assert_decide("1.2.3", "1.2.3", Never, channel, UpdateDecision::Skip);
                // 预发布通道切换
                assert_decide("1.2.0", "1.3.0-rc.1", Never, channel, UpdateDecision::Skip);
                // 预发布通道逃逸
                assert_decide("1.2.0-rc.1", "1.2.0", Never, channel, UpdateDecision::Skip);
                // 预发布通道递进
                assert_decide("1.2.0-rc.1", "1.2.0-rc.2", Never, channel, UpdateDecision::Skip);
            }
        }
    }

    /// 稳定通道下 latest 为预发布 → 一律跳过：升级 / 同号稳定→预发布，与 level 无关
    #[test]
    fn stable_channel_blocks_prerelease() {
        for level in [Patch, Minor, Major] {
            for downgrade in [false, true] {
                // 升级：稳定 → 预发布（通道切换被拦）
                assert_decide("1.2.1", "1.3.0-rc.1", level, Stable, UpdateDecision::Skip);
                // 预发布 → 预发布（递进也被拦：latest 是预发布即不可见）
                assert_decide("1.2.0-rc.1", "1.2.0-rc.2", level, Stable, UpdateDecision::Skip);
                // 同号稳定 → 预发布（semver 中 1.2.0-rc.1 < 1.2.0，属降级路径，门禁同样先拦）
                assert_decide("1.2.0", "1.2.0-rc.1", level, Stable, UpdateDecision::Skip);
            }
        }
    }

    /// 版本完全相同：无论 level / channel，一律跳过
    #[test]
    fn equal_versions_always_skip() {
        for level in [Patch, Minor, Major, Never] {
            for channel in [Stable, Prerelease] {
                assert_decide("1.2.3", "1.2.3", level, channel, UpdateDecision::Skip);
                assert_decide("1.2.3", "1.2.3", level, channel, UpdateDecision::Skip);
                assert_decide("1.2.3-alpha", "1.2.3-alpha", level, channel, UpdateDecision::Skip);
            }
        }
    }

    /// 预发布 → 稳定版：始终更新，不适用幅度门槛（与 level 无关）
    #[test]
    fn escape_prerelease_to_stable_always_updates() {
        for level in [Patch, Minor, Major] {
            for channel in [Prerelease, Stable] {
                // 同版本逃逸
                assert_decide("1.2.0-rc.1", "1.2.0", level, channel, UpdateDecision::Update);
                // 跨补丁逃逸
                assert_decide("1.2.0-rc.1", "1.2.1", level, channel, UpdateDecision::Update);
                // 跨次版本逃逸
                assert_decide("1.2.0-rc.1", "1.3.0", level, channel, UpdateDecision::Update);
                // 跨主版本逃逸
                assert_decide("1.2.0-rc.1", "2.0.0", level, channel, UpdateDecision::Update);
            }
        }
    }

    /// 预发布 → 预发布：跟随通道无条件更新，不适用幅度门槛（与 level 无关）
    #[test]
    fn prerelease_progression_skips_floor() {
        for level in [Patch, Minor, Major] {
            // 同号递进（rc.1 → rc.2）
            assert_decide("1.2.0-rc.1", "1.2.0-rc.2", level, Prerelease, UpdateDecision::Update);
            // 跨补丁递进
            assert_decide("1.2.0-rc.1", "1.2.1-rc.1", level, Prerelease, UpdateDecision::Update);
            // 跨次版本递进
            assert_decide("1.2.0-rc.1", "1.3.0-rc.1", level, Prerelease, UpdateDecision::Update);
            // 数字标识排序：alpha.2 → alpha.10 是升级（10 > 2，数字而非字典序）
            assert_decide("1.2.0-alpha.2", "1.2.0-alpha.10", level, Prerelease, UpdateDecision::Update);
        }
    }

    /// floor 全量表：3 种门槛 × 3 种数字变化（稳定 → 稳定）
    #[test]
    fn floor_matrix_stable_to_stable() {
        // Patch 门槛：任何升级都更新
        assert_decide("1.2.1", "1.2.2", Patch, Stable, UpdateDecision::Update);
        assert_decide("1.2.1", "1.3.0", Patch, Stable, UpdateDecision::Update);
        assert_decide("1.2.1", "2.0.0", Patch, Stable, UpdateDecision::Update);
        // Minor 门槛：忽略补丁
        assert_decide("1.2.1", "1.2.2", Minor, Stable, UpdateDecision::Skip);
        assert_decide("1.2.1", "1.3.0", Minor, Stable, UpdateDecision::Update);
        assert_decide("1.2.1", "2.0.0", Minor, Stable, UpdateDecision::Update);
        // Major 门槛：仅主版本
        assert_decide("1.2.1", "1.2.2", Major, Stable, UpdateDecision::Skip);
        assert_decide("1.2.1", "1.3.0", Major, Stable, UpdateDecision::Skip);
        assert_decide("1.2.1", "2.0.0", Major, Stable, UpdateDecision::Update);
    }

    /// 通道切换（稳定 → 预发布）：Prerelease 通道下仍按幅度门槛判断
    #[test]
    fn channel_switch_stable_to_prerelease_uses_floor() {
        // 补丁级切换（1.2.1 → 1.2.2-rc.1）
        assert_decide("1.2.1", "1.2.2-rc.1", Patch, Prerelease, UpdateDecision::Update);
        assert_decide("1.2.1", "1.2.2-rc.1", Minor, Prerelease, UpdateDecision::Skip);
        assert_decide("1.2.1", "1.2.2-rc.1", Major, Prerelease, UpdateDecision::Skip);
        // 次版本级切换（1.2.1 → 1.3.0-rc.1）
        assert_decide("1.2.1", "1.3.0-rc.1", Patch, Prerelease, UpdateDecision::Update);
        assert_decide("1.2.1", "1.3.0-rc.1", Minor, Prerelease, UpdateDecision::Update);
        assert_decide("1.2.1", "1.3.0-rc.1", Major, Prerelease, UpdateDecision::Skip);
        // 主版本级切换（1.2.1 → 2.0.0-rc.1）
        assert_decide("1.2.1", "2.0.0-rc.1", Patch, Prerelease, UpdateDecision::Update);
        assert_decide("1.2.1", "2.0.0-rc.1", Minor, Prerelease, UpdateDecision::Update);
        assert_decide("1.2.1", "2.0.0-rc.1", Major, Prerelease, UpdateDecision::Update);
    }

    /// semver crate 行为锁定：build metadata（+abc）是最后决胜键（规范偏离），
    /// 同号不同 build 判为 Greater → 幅度分类为 Patch → 按门槛判断
    #[test]
    fn build_metadata_counts_as_patch_change() {
        assert_decide("1.2.3+abc", "1.2.3+bcd", Patch, Stable, UpdateDecision::Update);
        assert_decide("1.2.3+abc", "1.2.3+bcd", Minor, Stable, UpdateDecision::Skip);
        // build metadata 不改变主/次/补丁幅度：1.2.3+abc → 1.2.4 仍是补丁升级
        assert_decide("1.2.3+abc", "1.2.4", Patch, Stable, UpdateDecision::Update);
        assert_decide("1.2.3+abc", "1.2.4", Minor, Stable, UpdateDecision::Skip);
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
        assert_eq!(DEFAULT_UPDATE_LEVEL, Patch);
        assert_eq!(DEFAULT_UPDATE_CHANNEL, Stable);
    }
}
