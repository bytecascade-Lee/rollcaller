//! 版本更新决策（纯函数，输入输出均为普通数据，便于单元测试，不依赖 Tauri）

use semver::Version;
use std::cmp::Ordering;
use ts_rs::TS;

// 原有的决策结果保持不变
#[derive(Debug, Clone, Copy, PartialEq, Eq, TS)]
pub enum UpdateDecision {
    Skip,
    Update,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TS)]
pub enum UpdatePolicy {
    /// 禁止所有更新（包括降级，若 allow_downgrade 为 true 也不更新）
    Never,
    /// 仅允许补丁更新，不允许预发布
    PatchNoPrerelease,
    /// 仅允许补丁更新，允许预发布（注意：semver 中同号预发布低于稳定版，
    /// 如 1.2.3-beta < 1.2.3，故"稳定版 -> 同号预发布"属于降级路径，需 allow_downgrade=true 才会更新）
    PatchWithPrerelease,
    /// 允许补丁和次版本更新，不允许预发布
    MinorNoPrerelease,
    /// 允许补丁和次版本更新，允许预发布
    MinorWithPrerelease,
    /// 允许主版本及以下所有更新，不允许预发布
    MajorNoPrerelease,
    /// 允许主版本及以下所有更新，允许预发布（所有非降级更新均可）
    MajorWithPrerelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VersionChange {
    Major,
    Minor,
    Patch,
    Prerelease, // 仅预发布标识变化（例如 1.2.3-alpha -> 1.2.3-beta）
    // 降级不在此列，单独处理
}

pub fn decide(current: &Version, latest: &Version, policy: UpdatePolicy, allow_downgrade: bool) -> UpdateDecision {
    // 1. 策略为 Never => 直接跳过
    if matches!(policy, UpdatePolicy::Never) {
        return UpdateDecision::Skip;
    }

    // 2. 预发布门禁：稳定版 -> 预发布，仅在策略允许预发布时通过
    let policy_allows_prerelease = matches!(
        policy,
        UpdatePolicy::PatchWithPrerelease | UpdatePolicy::MinorWithPrerelease | UpdatePolicy::MajorWithPrerelease
    );
    if !latest.pre.is_empty() && current.pre.is_empty() && !policy_allows_prerelease {
        return UpdateDecision::Skip;
    }

    // 3. 比较版本
    match latest.cmp(current) {
        Ordering::Equal => UpdateDecision::Skip,
        Ordering::Less => {
            // 降级：仅当显式允许且策略不为 Never（前面已排除）
            if allow_downgrade {
                UpdateDecision::Update
            } else {
                UpdateDecision::Skip
            }
        }
        Ordering::Greater => {
            // 计算实际变化级别
            let change = classify_version_change(current, latest);
            // 根据策略判断是否允许该变化
            if is_change_allowed(change, policy) {
                UpdateDecision::Update
            } else {
                UpdateDecision::Skip
            }
        }
    }
}

fn classify_version_change(current: &Version, latest: &Version) -> VersionChange {
    // 优先比较主/次/补丁号
    if latest.major > current.major {
        VersionChange::Major
    } else if latest.minor > current.minor {
        VersionChange::Minor
    } else if latest.patch > current.patch {
        VersionChange::Patch
    } else {
        // 主/次/补丁号相同，关注预发布标识
        // 若 current 是预发布，latest 是稳定版 => 视为 Patch（升级到稳定版）
        if latest.pre.is_empty() && !current.pre.is_empty() {
            VersionChange::Patch
        }
        // 若 current 是稳定版，latest 是预发布 => 视为 Prerelease（仅在策略允许时生效）
        else if !latest.pre.is_empty() && current.pre.is_empty() {
            VersionChange::Prerelease
        }
        // 两者都是预发布，且 pre 不同 => Prerelease
        else if !latest.pre.is_empty() && !current.pre.is_empty() && latest.pre != current.pre {
            VersionChange::Prerelease
        } else {
            // 完全一致（理论上不会走到这里）
            VersionChange::Patch
        }
    }
}

fn is_change_allowed(change: VersionChange, policy: UpdatePolicy) -> bool {
    use UpdatePolicy::*;
    use VersionChange::*;

    match policy {
        Never => false,
        PatchNoPrerelease => matches!(change, Patch),
        PatchWithPrerelease => matches!(change, Patch | Prerelease),
        MinorNoPrerelease => matches!(change, Patch | Minor),
        MinorWithPrerelease => matches!(change, Patch | Minor | Prerelease),
        MajorNoPrerelease => matches!(change, Patch | Minor | Major),
        MajorWithPrerelease => matches!(change, Patch | Minor | Major | Prerelease),
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

    /// Never 策略：无条件跳过，与版本关系、是否允许降级无关
    #[test]
    fn never_policy_skips_everything() {
        for allow_downgrade in [false, true] {
            assert_decide("1.0.0", "1.0.1", UpdatePolicy::Never, allow_downgrade, UpdateDecision::Skip); // 补丁升级
            assert_decide("1.0.0", "1.1.0", UpdatePolicy::Never, allow_downgrade, UpdateDecision::Skip); // 次版本升级
            assert_decide("1.0.0", "2.0.0", UpdatePolicy::Never, allow_downgrade, UpdateDecision::Skip); // 主版本升级
            assert_decide("1.0.0", "1.0.0-beta", UpdatePolicy::Never, allow_downgrade, UpdateDecision::Skip); // 升级到预发布
            assert_decide("2.0.0", "1.0.0", UpdatePolicy::Never, allow_downgrade, UpdateDecision::Skip); // 降级
            assert_decide("1.0.0", "1.0.0", UpdatePolicy::Never, allow_downgrade, UpdateDecision::Skip); // 相同版本
        }
    }

    /// 版本完全相同：无论策略、是否允许降级，一律跳过
    #[test]
    fn equal_versions_always_skip() {
        for policy in [
            UpdatePolicy::Never,
            UpdatePolicy::PatchNoPrerelease,
            UpdatePolicy::PatchWithPrerelease,
            UpdatePolicy::MinorNoPrerelease,
            UpdatePolicy::MinorWithPrerelease,
            UpdatePolicy::MajorNoPrerelease,
            UpdatePolicy::MajorWithPrerelease,
        ] {
            assert_decide("1.2.3", "1.2.3", policy, false, UpdateDecision::Skip);
            assert_decide("1.2.3", "1.2.3", policy, true, UpdateDecision::Skip);
            assert_decide("1.2.3-alpha", "1.2.3-alpha", policy, false, UpdateDecision::Skip);
        }
    }

    /// build metadata（+abc）在 semver crate 中的比较行为：
    /// 规范规定 precedence 忽略 build metadata，但 semver crate 的 Ord 将其作为最后 tiebreaker——
    /// 故 1.2.3+abc 与 1.2.3+bcd 不相等，主/次/补丁号相同会归类为 Patch 并走更新路径
    #[test]
    fn build_metadata_acts_as_ord_tiebreaker() {
        // 仅 build metadata 不同 → semver crate 判为 Greater（tiebreaker）→ Patch → 更新
        assert_decide("1.2.3+abc", "1.2.3+bcd", UpdatePolicy::MajorNoPrerelease, false, UpdateDecision::Update);
        assert_decide("1.2.3+abc", "1.2.3+bcd", UpdatePolicy::PatchNoPrerelease, false, UpdateDecision::Update);
        // 预发布 + 仅 build metadata 不同：同样视为 Patch（current 已是预发布，门禁不生效）
        assert_decide(
            "1.2.3-alpha+abc",
            "1.2.3-alpha+bcd",
            UpdatePolicy::PatchNoPrerelease,
            false,
            UpdateDecision::Update,
        );
        // build metadata 不影响主/次/补丁号比较：1.2.3+abc -> 1.2.4 仍是补丁升级
        assert_decide("1.2.3+abc", "1.2.4", UpdatePolicy::PatchNoPrerelease, false, UpdateDecision::Update);
    }

    /// 补丁升级（1.2.3 -> 1.2.4）：所有非 Never 策略均允许
    #[test]
    fn patch_update_allowed_by_all_non_never_policies() {
        for policy in [
            UpdatePolicy::PatchNoPrerelease,
            UpdatePolicy::PatchWithPrerelease,
            UpdatePolicy::MinorNoPrerelease,
            UpdatePolicy::MinorWithPrerelease,
            UpdatePolicy::MajorNoPrerelease,
            UpdatePolicy::MajorWithPrerelease,
        ] {
            assert_decide("1.2.3", "1.2.4", policy, false, UpdateDecision::Update);
        }
    }

    /// 次版本升级（1.2.3 -> 1.3.0）：需要 Minor 及以上策略
    #[test]
    fn minor_update_requires_minor_or_major_policy() {
        assert_decide("1.2.3", "1.3.0", UpdatePolicy::PatchNoPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3", "1.3.0", UpdatePolicy::PatchWithPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3", "1.3.0", UpdatePolicy::MinorNoPrerelease, false, UpdateDecision::Update);
        assert_decide("1.2.3", "1.3.0", UpdatePolicy::MinorWithPrerelease, false, UpdateDecision::Update);
        assert_decide("1.2.3", "1.3.0", UpdatePolicy::MajorNoPrerelease, false, UpdateDecision::Update);
        assert_decide("1.2.3", "1.3.0", UpdatePolicy::MajorWithPrerelease, false, UpdateDecision::Update);
    }

    /// 主版本升级（1.2.3 -> 2.0.0）：仅 Major 策略允许
    #[test]
    fn major_update_requires_major_policy() {
        assert_decide("1.2.3", "2.0.0", UpdatePolicy::PatchNoPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3", "2.0.0", UpdatePolicy::PatchWithPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3", "2.0.0", UpdatePolicy::MinorNoPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3", "2.0.0", UpdatePolicy::MinorWithPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3", "2.0.0", UpdatePolicy::MajorNoPrerelease, false, UpdateDecision::Update);
        assert_decide("1.2.3", "2.0.0", UpdatePolicy::MajorWithPrerelease, false, UpdateDecision::Update);
    }

    /// 降级：仅当显式允许时才更新，且不受变化级别策略约束（Never 除外，已单独覆盖）
    #[test]
    fn downgrade_respects_allow_downgrade_flag() {
        // 跨主版本降级：即使不满足 Minor 策略，允许降级时仍可更新
        assert_decide("1.3.0", "1.2.3", UpdatePolicy::MinorNoPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.3.0", "1.2.3", UpdatePolicy::MinorNoPrerelease, true, UpdateDecision::Update);
        // 普通补丁级降级
        assert_decide("1.2.4", "1.2.3", UpdatePolicy::PatchNoPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.4", "1.2.3", UpdatePolicy::PatchNoPrerelease, true, UpdateDecision::Update);
        // 主版本降级
        assert_decide("2.0.0", "1.0.0", UpdatePolicy::MajorWithPrerelease, false, UpdateDecision::Skip);
        assert_decide("2.0.0", "1.0.0", UpdatePolicy::MajorWithPrerelease, true, UpdateDecision::Update);
    }

    /// 稳定版 -> 预发布：NoPrerelease 策略一律拦截（与版本跨度无关）；
    /// WithPrerelease 策略放行，但仍需满足变化级别限制
    #[test]
    fn stable_to_prerelease_blocked_without_prerelease_policy() {
        // NoPrerelease 策略：全部跳过
        assert_decide("1.2.3", "1.2.3-beta", UpdatePolicy::PatchNoPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3", "1.2.4-beta", UpdatePolicy::PatchNoPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3", "1.3.0-beta", UpdatePolicy::MinorNoPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3", "2.0.0-beta", UpdatePolicy::MajorNoPrerelease, false, UpdateDecision::Skip);
        // WithPrerelease 策略：与变化级别匹配时更新。
        // 注意 semver 语义：同号预发布低于稳定版（1.2.3-beta < 1.2.3），
        // 因此“稳定版 -> 同号预发布”在版本比较上属于降级，仅 allow_downgrade=true 时才更新
        assert_decide("1.2.3", "1.2.3-beta", UpdatePolicy::PatchWithPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3", "1.2.3-beta", UpdatePolicy::PatchWithPrerelease, true, UpdateDecision::Update);
        // 补丁号升高（1.2.3 -> 1.2.4-beta）是真正的升级，Patch 策略即可放行
        assert_decide("1.2.3", "1.2.4-beta", UpdatePolicy::PatchWithPrerelease, false, UpdateDecision::Update);
        assert_decide("1.2.3", "1.3.0-beta", UpdatePolicy::MinorWithPrerelease, false, UpdateDecision::Update);
        assert_decide("1.2.3", "2.0.0-beta", UpdatePolicy::MajorWithPrerelease, false, UpdateDecision::Update);
        // 变化级别超出策略允许范围时仍然跳过
        assert_decide("1.2.3", "1.3.0-beta", UpdatePolicy::PatchWithPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3", "2.0.0-beta", UpdatePolicy::PatchWithPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3", "2.0.0-beta", UpdatePolicy::MinorWithPrerelease, false, UpdateDecision::Skip);
    }

    /// 预发布 -> 同号稳定版：视为 Patch 升级，所有非 Never 策略均允许
    #[test]
    fn prerelease_to_stable_is_patch_upgrade() {
        for policy in [
            UpdatePolicy::PatchNoPrerelease,
            UpdatePolicy::PatchWithPrerelease,
            UpdatePolicy::MinorNoPrerelease,
            UpdatePolicy::MinorWithPrerelease,
            UpdatePolicy::MajorNoPrerelease,
            UpdatePolicy::MajorWithPrerelease,
        ] {
            assert_decide("1.2.3-beta", "1.2.3", policy, false, UpdateDecision::Update);
            assert_decide("1.2.3-alpha", "1.2.3", policy, false, UpdateDecision::Update);
        }
    }

    /// 主/次/补丁号相同、仅预发布标识变化：只有允许预发布的策略才更新
    #[test]
    fn prerelease_to_prerelease_same_numbers() {
        assert_decide("1.2.3-alpha", "1.2.3-beta", UpdatePolicy::PatchNoPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3-alpha", "1.2.3-beta", UpdatePolicy::MinorNoPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3-alpha", "1.2.3-beta", UpdatePolicy::MajorNoPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3-alpha", "1.2.3-beta", UpdatePolicy::PatchWithPrerelease, false, UpdateDecision::Update);
        assert_decide("1.2.3-alpha", "1.2.3-beta", UpdatePolicy::MinorWithPrerelease, false, UpdateDecision::Update);
        assert_decide("1.2.3-alpha", "1.2.3-beta", UpdatePolicy::MajorWithPrerelease, false, UpdateDecision::Update);
        // 数字型预发布标识按数值比较：alpha.2 -> alpha.10 仍属 Prerelease
        assert_decide("1.2.3-alpha.2", "1.2.3-alpha.10", UpdatePolicy::PatchNoPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3-alpha.2", "1.2.3-alpha.10", UpdatePolicy::PatchWithPrerelease, false, UpdateDecision::Update);
    }

    /// current 已是预发布时门禁不生效，预发布 -> 更高版本按变化级别判断
    #[test]
    fn prerelease_to_higher_version_follows_change_level() {
        // 预发布 -> 更高补丁号的预发布 => Patch
        assert_decide("1.2.3-alpha", "1.2.4-beta", UpdatePolicy::PatchNoPrerelease, false, UpdateDecision::Update);
        // 预发布 -> 更高次版本的预发布 => Minor
        assert_decide("1.2.3-alpha", "1.3.0-beta", UpdatePolicy::PatchNoPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3-alpha", "1.3.0-beta", UpdatePolicy::MinorNoPrerelease, false, UpdateDecision::Update);
        // 预发布 -> 更高主版本的预发布 => Major
        assert_decide("1.2.3-alpha", "2.0.0-beta", UpdatePolicy::MinorNoPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3-alpha", "2.0.0-beta", UpdatePolicy::MajorNoPrerelease, false, UpdateDecision::Update);
    }

    /// 预发布之间的“降级”（如 beta -> alpha）：遵循降级规则而非变化级别规则
    #[test]
    fn prerelease_downgrade_follows_downgrade_rules() {
        assert_decide("1.2.3-beta", "1.2.3-alpha", UpdatePolicy::MajorWithPrerelease, false, UpdateDecision::Skip);
        assert_decide("1.2.3-beta", "1.2.3-alpha", UpdatePolicy::MajorWithPrerelease, true, UpdateDecision::Update);
    }

    /// classify_version_change 各分支
    #[test]
    fn classify_version_change_branches() {
        assert_eq!(classify_version_change(&v("1.2.3"), &v("2.0.0")), VersionChange::Major);
        assert_eq!(classify_version_change(&v("1.2.3"), &v("1.3.0")), VersionChange::Minor);
        assert_eq!(classify_version_change(&v("1.2.3"), &v("1.2.4")), VersionChange::Patch);
        // 预发布 -> 稳定版（同号）：视为 Patch
        assert_eq!(classify_version_change(&v("1.2.3-alpha"), &v("1.2.3")), VersionChange::Patch);
        // 稳定版 -> 预发布（同号）：视为 Prerelease
        assert_eq!(classify_version_change(&v("1.2.3"), &v("1.2.3-beta")), VersionChange::Prerelease);
        // 预发布 -> 预发布（同号不同标识）：视为 Prerelease
        assert_eq!(classify_version_change(&v("1.2.3-alpha"), &v("1.2.3-beta")), VersionChange::Prerelease);
        // 数字型预发布标识按数值比较
        assert_eq!(
            classify_version_change(&v("1.2.3-alpha.2"), &v("1.2.3-alpha.10")),
            VersionChange::Prerelease
        );
        // 完全一致（decide 的 Equal 分支已拦截，函数契约上回到 Patch）
        assert_eq!(classify_version_change(&v("1.2.3"), &v("1.2.3")), VersionChange::Patch);
        assert_eq!(
            classify_version_change(&v("1.2.3-alpha"), &v("1.2.3-alpha")),
            VersionChange::Patch
        );
    }

    /// is_change_allowed 全量组合表：7 种策略 x 4 种变化级别
    #[test]
    fn is_change_allowed_table() {
        use UpdatePolicy::*;
        use VersionChange::*;
        let cases: &[(VersionChange, UpdatePolicy, bool)] = &[
            // Never：一律禁止
            (Major, Never, false),
            (Minor, Never, false),
            (Patch, Never, false),
            (Prerelease, Never, false),
            // PatchNoPrerelease：仅 Patch
            (Patch, PatchNoPrerelease, true),
            (Minor, PatchNoPrerelease, false),
            (Major, PatchNoPrerelease, false),
            (Prerelease, PatchNoPrerelease, false),
            // PatchWithPrerelease：Patch + Prerelease
            (Patch, PatchWithPrerelease, true),
            (Minor, PatchWithPrerelease, false),
            (Major, PatchWithPrerelease, false),
            (Prerelease, PatchWithPrerelease, true),
            // MinorNoPrerelease：Patch + Minor
            (Patch, MinorNoPrerelease, true),
            (Minor, MinorNoPrerelease, true),
            (Major, MinorNoPrerelease, false),
            (Prerelease, MinorNoPrerelease, false),
            // MinorWithPrerelease：Patch + Minor + Prerelease
            (Patch, MinorWithPrerelease, true),
            (Minor, MinorWithPrerelease, true),
            (Major, MinorWithPrerelease, false),
            (Prerelease, MinorWithPrerelease, true),
            // MajorNoPrerelease：Patch + Minor + Major
            (Patch, MajorNoPrerelease, true),
            (Minor, MajorNoPrerelease, true),
            (Major, MajorNoPrerelease, true),
            (Prerelease, MajorNoPrerelease, false),
            // MajorWithPrerelease：全部允许
            (Patch, MajorWithPrerelease, true),
            (Minor, MajorWithPrerelease, true),
            (Major, MajorWithPrerelease, true),
            (Prerelease, MajorWithPrerelease, true),
        ];
        for (change, policy, expected) in cases {
            assert_eq!(
                is_change_allowed(*change, *policy),
                *expected,
                "is_change_allowed(change={:?}, policy={:?})",
                change,
                policy
            );
        }
    }
}
