//! 版本更新决策（纯函数，输入输出均为普通数据，便于单元测试，不依赖 Tauri）
//!
//! 策略语义为「最小可接受更新单元」（floor），而非依赖库场景的「最大容忍变化量」（cap）：
//! 桌面软件没有 API 兼容包袱，用户关心的不是"变化太大"，而是"更新太频繁、每次都要
//! 确认/重启太烦"。因此用户设定的等级表示：至少要有这么大的变化才值得更新，
//! 低于门槛的更新被忽略（延迟聚合，等下一个达标的版本一次跳过去）。

use semver::Version;
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

/// 更新策略 = 幅度门槛（floor）+ 更新通道（stable / prerelease）
///
/// ## 幅度门槛（floor）
///
/// | 门槛 | 1.2.1→1.2.2 | 1.2.1→1.3.0 | 1.2.1→2.0.0 |
/// |---|---|---|---|
/// | Patch | 更新 | 更新 | 更新 |
/// | Minor | 跳过 | 更新 | 更新 |
/// | Major | 跳过 | 跳过 | 更新 |
///
/// ## 更新通道
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, TS)]
pub enum UpdatePolicy {
    /// 禁止所有更新（包括降级，若 allow_downgrade 为 true 也不更新）
    Never,
    /// 幅度门槛 Patch + 稳定通道：任何稳定版升级都更新（最积极）
    PatchStable,
    /// 幅度门槛 Patch + 预发布通道
    PatchPrerelease,
    /// 幅度门槛 Minor + 稳定通道：忽略补丁更新，次版本及以上才更新
    MinorStable,
    /// 幅度门槛 Minor + 预发布通道
    MinorPrerelease,
    /// 幅度门槛 Major + 稳定通道：仅主版本更新
    MajorStable,
    /// 幅度门槛 Major + 预发布通道
    MajorPrerelease,
}

impl UpdatePolicy {
    /// 该策略的幅度门槛（Never 无门槛）
    fn floor(self) -> Option<UpdateLevel> {
        use UpdatePolicy::*;
        match self {
            Never => None,
            PatchStable | PatchPrerelease => Some(UpdateLevel::Patch),
            MinorStable | MinorPrerelease => Some(UpdateLevel::Minor),
            MajorStable | MajorPrerelease => Some(UpdateLevel::Major),
        }
    }

    /// 是否允许预发布版本（稳定通道 → false）
    fn allows_prerelease(self) -> bool {
        use UpdatePolicy::*;
        matches!(self, PatchPrerelease | MinorPrerelease | MajorPrerelease)
    }
}

/// 变化幅度：升级路径中只比较主/次/补丁号（预发布标识不参与幅度分类）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VersionChange {
    Major,
    Minor,
    Patch,
}

/// 幅度门槛（floor）：最小可接受的更新单元
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateLevel {
    /// 补丁及以上
    Patch,
    /// 次版本及以上
    Minor,
    /// 仅主版本
    Major,
}

/// 版本更新决策入口。
///
/// 参数均为普通数据（无 Tauri 依赖），便于单元测试：
/// - `current` / `latest`：当前版本与清单中的目标版本
/// - `policy`：更新策略（幅度门槛 + 通道）
/// - `allow_downgrade`：是否允许降级（与幅度门槛/通道正交，显式允许才放行）
pub fn decide(current: &Version, latest: &Version, policy: UpdatePolicy, allow_downgrade: bool) -> UpdateDecision {
    // 1. Never：无条件跳过
    if policy == UpdatePolicy::Never {
        return UpdateDecision::Skip;
    }

    // 2. 通道门禁：latest 为预发布版本且策略不允许预发布 → 一律跳过（升级/降级同判，
    //    保证稳定通道用户不会接触到任何预发布版本）
    let latest_is_pre = !latest.pre.is_empty();
    if latest_is_pre && !policy.allows_prerelease() {
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
        Ordering::Greater => decide_upgrade(current, latest, policy),
    }
}

/// 升级路径的决策：处理预发布的三种特殊形态后，按幅度门槛（floor）判断
fn decide_upgrade(current: &Version, latest: &Version, policy: UpdatePolicy) -> UpdateDecision {
    let current_is_pre = !current.pre.is_empty();
    let latest_is_pre = !latest.pre.is_empty();

    // 逃逸通道：预发布 → 稳定版，始终更新（不适用幅度门槛，否则用户永远卡在预发布）
    if current_is_pre && !latest_is_pre {
        return UpdateDecision::Update;
    }
    // 通道内递进：预发布 → 预发布，跟随通道（不适用幅度门槛，风险已在进入通道时接受；
    // 通道门禁已在上游确保当前策略允许预发布）
    if current_is_pre && latest_is_pre {
        return UpdateDecision::Update;
    }
    // 其余：稳定 → 稳定，或稳定 → 预发布的通道切换 → 按幅度门槛判断
    let change = classify_numeric_change(current, latest);
    let floor = policy.floor().expect("非 Never 策略必有幅度门槛");
    if floor_satisfied(change, floor) {
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

    /// 便捷解析：测试中的字面量均为合法 semver，解析失败直接 panic
    fn v(s: &str) -> Version {
        Version::parse(s).expect("测试用例必须是合法 semver")
    }

    /// decide 的断言助手：失败时打印完整上下文，便于定位具体组合
    fn assert_decide(
        current: &str,
        latest: &str,
        policy: UpdatePolicy,
        allow_downgrade: bool,
        expected: UpdateDecision,
    ) {
        let got = decide(&v(current), &v(latest), policy, allow_downgrade);
        assert_eq!(
            got,
            expected,
            "decide(current={}, latest={}, policy={:?}, allow_downgrade={})",
            current,
            latest,
            policy,
            allow_downgrade
        );
    }

    const ALL_POLICIES: [UpdatePolicy; 7] = [
        UpdatePolicy::Never,
        UpdatePolicy::PatchStable,
        UpdatePolicy::PatchPrerelease,
        UpdatePolicy::MinorStable,
        UpdatePolicy::MinorPrerelease,
        UpdatePolicy::MajorStable,
        UpdatePolicy::MajorPrerelease,
    ];
    /// 非 Never 的全部策略
    const ACTIVE_POLICIES: [UpdatePolicy; 6] = [
        UpdatePolicy::PatchStable,
        UpdatePolicy::PatchPrerelease,
        UpdatePolicy::MinorStable,
        UpdatePolicy::MinorPrerelease,
        UpdatePolicy::MajorStable,
        UpdatePolicy::MajorPrerelease,
    ];
    /// 稳定通道的全部策略
    const STABLE_POLICIES: [UpdatePolicy; 3] = [
        UpdatePolicy::PatchStable,
        UpdatePolicy::MinorStable,
        UpdatePolicy::MajorStable,
    ];
    /// 预发布通道的全部策略
    const PRERELEASE_POLICIES: [UpdatePolicy; 3] = [
        UpdatePolicy::PatchPrerelease,
        UpdatePolicy::MinorPrerelease,
        UpdatePolicy::MajorPrerelease,
    ];

    /// Never 策略：无条件跳过，与版本关系、是否允许降级、通道均无关
    #[test]
    fn never_policy_skips_everything() {
        for allow_downgrade in [false, true] {
            assert_decide("1.0.0", "1.0.1", UpdatePolicy::Never, allow_downgrade, UpdateDecision::Skip); // 补丁升级
            assert_decide("1.0.0", "1.1.0", UpdatePolicy::Never, allow_downgrade, UpdateDecision::Skip); // 次版本升级
            assert_decide("1.0.0", "2.0.0", UpdatePolicy::Never, allow_downgrade, UpdateDecision::Skip); // 主版本升级
            assert_decide("1.0.0", "1.0.0-beta", UpdatePolicy::Never, allow_downgrade, UpdateDecision::Skip); // 升级到预发布
            assert_decide("1.0.0-beta", "1.0.0", UpdatePolicy::Never, allow_downgrade, UpdateDecision::Skip); // 预发布 -> 稳定版
            assert_decide("2.0.0", "1.0.0", UpdatePolicy::Never, allow_downgrade, UpdateDecision::Skip); // 降级
            assert_decide("1.0.0", "1.0.0", UpdatePolicy::Never, allow_downgrade, UpdateDecision::Skip); // 相同版本
        }
    }

    /// 版本完全相同：无论策略、是否允许降级，一律跳过
    #[test]
    fn equal_versions_always_skip() {
        for policy in ALL_POLICIES {
            assert_decide("1.2.3", "1.2.3", policy, false, UpdateDecision::Skip);
            assert_decide("1.2.3", "1.2.3", policy, true, UpdateDecision::Skip);
            assert_decide("1.2.3-alpha", "1.2.3-alpha", policy, false, UpdateDecision::Skip);
        }
    }

    /// floor=Patch：任何稳定版升级（补丁/次版本/主版本）都更新
    #[test]
    fn floor_patch_updates_all_stable_upgrades() {
        for policy in [UpdatePolicy::PatchStable, UpdatePolicy::PatchPrerelease] {
            assert_decide("1.2.3", "1.2.4", policy, false, UpdateDecision::Update);
            assert_decide("1.2.3", "1.3.0", policy, false, UpdateDecision::Update);
            assert_decide("1.2.3", "2.0.0", policy, false, UpdateDecision::Update);
        }
    }

    /// floor=Minor：忽略补丁更新，次版本及以上才更新 —— 核心语义
    #[test]
    fn floor_minor_skips_patch_updates() {
        for policy in [UpdatePolicy::MinorStable, UpdatePolicy::MinorPrerelease] {
            assert_decide("1.2.1", "1.2.2", policy, false, UpdateDecision::Skip); // 补丁：跳过
            assert_decide("1.2.1", "1.3.0", policy, false, UpdateDecision::Update); // 次版本：更新
            assert_decide("1.2.1", "2.0.0", policy, false, UpdateDecision::Update); // 主版本：更新
        }
    }

    /// floor=Major：仅主版本更新
    #[test]
    fn floor_major_requires_major() {
        for policy in [UpdatePolicy::MajorStable, UpdatePolicy::MajorPrerelease] {
            assert_decide("1.2.3", "1.2.4", policy, false, UpdateDecision::Skip);
            assert_decide("1.2.3", "1.3.0", policy, false, UpdateDecision::Skip);
            assert_decide("1.2.3", "2.0.0", policy, false, UpdateDecision::Update);
        }
    }

    /// 降级：仅当显式允许时才更新，不受幅度门槛约束（Never 除外，已单独覆盖）
    #[test]
    fn downgrade_respects_allow_downgrade_flag() {
        // 次版本级降级：即使满足 Minor 门槛也不更新（降级是另一条路径）
        assert_decide("1.3.0", "1.2.3", UpdatePolicy::MinorStable, false, UpdateDecision::Skip);
        assert_decide("1.3.0", "1.2.3", UpdatePolicy::MinorStable, true, UpdateDecision::Update);
        // 补丁级降级
        assert_decide("1.2.4", "1.2.3", UpdatePolicy::PatchStable, false, UpdateDecision::Skip);
        assert_decide("1.2.4", "1.2.3", UpdatePolicy::PatchStable, true, UpdateDecision::Update);
        // 主版本降级
        assert_decide("2.0.0", "1.0.0", UpdatePolicy::MajorPrerelease, false, UpdateDecision::Skip);
        assert_decide("2.0.0", "1.0.0", UpdatePolicy::MajorPrerelease, true, UpdateDecision::Update);
    }

    /// 稳定通道：latest 为预发布版本时一律跳过（无论幅度、无论是否允许降级）
    #[test]
    fn stable_channel_blocks_all_prerelease_versions() {
        for policy in STABLE_POLICIES {
            for allow_downgrade in [false, true] {
                // 同号预发布（semver 上属降级路径）也要被通道门禁拦下
                assert_decide("1.2.3", "1.2.3-beta", policy, allow_downgrade, UpdateDecision::Skip);
                // 升级到预发布
                assert_decide("1.2.3", "1.2.4-beta", policy, allow_downgrade, UpdateDecision::Skip);
                assert_decide("1.2.3", "1.3.0-beta", policy, allow_downgrade, UpdateDecision::Skip);
                assert_decide("1.2.3", "2.0.0-beta", policy, allow_downgrade, UpdateDecision::Skip);
                // 预发布之间的递进同样拦截
                assert_decide("1.2.3-alpha", "1.2.3-beta", policy, allow_downgrade, UpdateDecision::Skip);
            }
        }
    }

    /// 稳定版 -> 预发布（通道切换）：需要预发布通道，且仍按幅度门槛判断
    #[test]
    fn stable_to_prerelease_switch_applies_floor() {
        // 补丁级预发布（1.2.3 -> 1.2.4-beta）：仅 floor=Patch 放行
        assert_decide("1.2.3", "1.2.4-beta", UpdatePolicy::PatchPrerelease, false, UpdateDecision::Update);
        assert_decide("1.2.3", "1.2.4-beta", UpdatePolicy::MinorPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3", "1.2.4-beta", UpdatePolicy::MajorPrerelease, false, UpdateDecision::Skip);
        // 次版本级预发布（1.2.3 -> 1.3.0-beta）：floor 不低于 Minor 才放行
        assert_decide("1.2.3", "1.3.0-beta", UpdatePolicy::PatchPrerelease, false, UpdateDecision::Update);
        assert_decide("1.2.3", "1.3.0-beta", UpdatePolicy::MinorPrerelease, false, UpdateDecision::Update);
        assert_decide("1.2.3", "1.3.0-beta", UpdatePolicy::MajorPrerelease, false, UpdateDecision::Skip);
        // 主版本级预发布（1.2.3 -> 2.0.0-beta）：全通道放行
        assert_decide("1.2.3", "2.0.0-beta", UpdatePolicy::PatchPrerelease, false, UpdateDecision::Update);
        assert_decide("1.2.3", "2.0.0-beta", UpdatePolicy::MinorPrerelease, false, UpdateDecision::Update);
        assert_decide("1.2.3", "2.0.0-beta", UpdatePolicy::MajorPrerelease, false, UpdateDecision::Update);
    }

    /// 通道内递进（预发布 -> 预发布）：跟随通道，绕过幅度门槛，一律更新
    #[test]
    fn prerelease_progression_follows_channel_bypassing_floor() {
        for policy in PRERELEASE_POLICIES {
            // 同号标识变化：1.2.3-alpha -> 1.2.3-beta
            assert_decide("1.2.3-alpha", "1.2.3-beta", policy, false, UpdateDecision::Update);
            // 数字型标识按数值比较：alpha.2 -> alpha.10
            assert_decide("1.2.3-alpha.2", "1.2.3-alpha.10", policy, false, UpdateDecision::Update);
            // 更高补丁/次版本/主版本的预发布：同样是通道内递进，不受 floor 约束
            assert_decide("1.2.3-alpha", "1.2.4-beta", policy, false, UpdateDecision::Update);
            assert_decide("1.2.3-alpha", "1.3.0-beta", policy, false, UpdateDecision::Update);
            assert_decide("1.2.3-alpha", "2.0.0-beta", policy, false, UpdateDecision::Update);
        }
    }

    /// 逃逸通道（预发布 -> 稳定版）：始终更新，不受幅度门槛与通道约束
    #[test]
    fn prerelease_to_stable_always_updates() {
        for policy in ACTIVE_POLICIES {
            // 同号稳定版（1.2.3-beta -> 1.2.3）：必须更新，否则永远卡在预发布
            assert_decide("1.2.3-beta", "1.2.3", policy, false, UpdateDecision::Update);
            // 更高补丁/主版本稳定版
            assert_decide("1.2.3-beta", "1.2.4", policy, false, UpdateDecision::Update);
            assert_decide("1.2.3-beta", "2.0.0", policy, false, UpdateDecision::Update);
        }
    }

    /// 预发布之间的"降级"（如 beta -> alpha）：遵循降级规则而非通道内递进
    #[test]
    fn prerelease_downgrade_follows_downgrade_rules() {
        assert_decide("1.2.3-beta", "1.2.3-alpha", UpdatePolicy::MajorPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3-beta", "1.2.3-alpha", UpdatePolicy::MajorPrerelease, true, UpdateDecision::Update);
    }

    /// build metadata（+abc）在 semver crate 中作为最后决胜键（规范偏离）：
    /// 1.2.3+abc -> 1.2.3+bcd 判为 Greater，幅度上等同补丁 → 受 floor 约束
    #[test]
    fn build_metadata_tiebreaker_subject_to_floor() {
        // 仅 build metadata 不同 → crate 判为 Greater → 幅度 Patch → floor=Patch 才更新
        assert_decide("1.2.3+abc", "1.2.3+bcd", UpdatePolicy::PatchStable, false, UpdateDecision::Update);
        assert_decide("1.2.3+abc", "1.2.3+bcd", UpdatePolicy::MinorStable, false, UpdateDecision::Skip);
        assert_decide("1.2.3+abc", "1.2.3+bcd", UpdatePolicy::MajorStable, false, UpdateDecision::Skip);
        // build metadata 不影响主/次/补丁号比较：1.2.3+abc -> 1.2.4 仍是补丁升级
        assert_decide("1.2.3+abc", "1.2.4", UpdatePolicy::PatchStable, false, UpdateDecision::Update);
        // 完全相同的 build metadata 视为相同版本
        assert_decide("1.2.3+abc", "1.2.3+abc", UpdatePolicy::PatchStable, true, UpdateDecision::Skip);
    }

    /// floor_satisfied 全量组合表：3 种门槛 x 3 种变化幅度
    #[test]
    fn floor_satisfied_table() {
        use VersionChange::*;
        let cases: &[(VersionChange, UpdateLevel, bool)] = &[
            // Patch 门槛：任何升级都允许
            (Patch, UpdateLevel::Patch, true),
            (Minor, UpdateLevel::Patch, true),
            (Major, UpdateLevel::Patch, true),
            // Minor 门槛：忽略补丁
            (Patch, UpdateLevel::Minor, false),
            (Minor, UpdateLevel::Minor, true),
            (Major, UpdateLevel::Minor, true),
            // Major 门槛：仅主版本
            (Patch, UpdateLevel::Major, false),
            (Minor, UpdateLevel::Major, false),
            (Major, UpdateLevel::Major, true),
        ];
        for (change, floor, expected) in cases {
            assert_eq!(
                floor_satisfied(*change, *floor),
                *expected,
                "floor_satisfied(change={:?}, floor={:?})",
                change,
                floor
            );
        }
    }

    /// classify_numeric_change 各分支：只比较主/次/补丁号，预发布标识不参与
    #[test]
    fn classify_numeric_change_branches() {
        assert_eq!(classify_numeric_change(&v("1.2.3"), &v("2.0.0")), VersionChange::Major);
        assert_eq!(classify_numeric_change(&v("1.2.3"), &v("1.3.0")), VersionChange::Minor);
        assert_eq!(classify_numeric_change(&v("1.2.3"), &v("1.2.4")), VersionChange::Patch);
        // 预发布版本同样只按数字分类（通道内递进已在上游接管，这里仅验证分类本身）
        assert_eq!(classify_numeric_change(&v("1.2.3"), &v("1.2.4-beta")), VersionChange::Patch);
        assert_eq!(classify_numeric_change(&v("1.2.3"), &v("1.3.0-beta")), VersionChange::Minor);
        assert_eq!(classify_numeric_change(&v("1.2.3-alpha"), &v("2.0.0-beta")), VersionChange::Major);
    }
}
