//! 版本更新决策

use crate::updater::model::{Policy, Severity, SkipReason, UpdateChannel, UpdateDecision, UpdateLevel, UpdateReason};
use semver::Version;

/// 更新判定的唯一入口
pub fn decide(current: &Version, latest: &Version, policy: &Policy, severity: Severity) -> UpdateDecision {
    fuse(verdict(current, latest, policy), severity)
}

/// 只认版本 × 策略的判定（severity 尚未参与）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// 被与 severity 无关的门禁挡住
    /// 只可能是 `Disabled` / `ChannelBlocked` / `NotNewer`
    Blocked(SkipReason),
    /// 预发布 -> 稳定版，门槛不适用
    PrereleaseEscape,
    /// 预发布 -> 预发布，门槛不适用
    PrereleaseProgression,
    /// 数字幅度达标
    FloorMet,
    /// 数字幅度不足
    BelowFloor,
}

/// 门禁按序关闭：通道 -> 方向 -> 策略 -> 预发布形态 -> 门槛
fn verdict(current: &Version, latest: &Version, policy: &Policy) -> Verdict {
    if channel_blocks(policy.channel, latest) {
        return Verdict::Blocked(SkipReason::ChannelBlocked);
    }
    if !is_newer(current, latest) {
        return Verdict::Blocked(SkipReason::NotNewer);
    }
    if policy.level == UpdateLevel::Never {
        return Verdict::Blocked(SkipReason::Disabled);
    }
    if escape_to_stable(current, latest) {
        return Verdict::PrereleaseEscape;
    }
    if prerelease_progression(current, latest) {
        return Verdict::PrereleaseProgression;
    }
    if meets_floor(numeric_change(current, latest), policy.level) {
        Verdict::FloorMet
    } else {
        Verdict::BelowFloor
    }
}

/// severity 融合
fn fuse(verdict: Verdict, severity: Severity) -> UpdateDecision {
    use UpdateDecision::{Skip, Update};
    use UpdateReason::*;

    match (verdict, severity) {
        (Verdict::Blocked(SkipReason::Disabled), Severity::Critical) => Update(Force),
        (Verdict::Blocked(reason), _) => Skip(reason),
        (Verdict::BelowFloor, Severity::Normal) => Skip(SkipReason::BelowFloor),
        (Verdict::BelowFloor, _) => Update(Exempt),
        (Verdict::FloorMet, _) => Update(Newer),
        (Verdict::PrereleaseEscape, _) => Update(PrereleaseEscape),
        (Verdict::PrereleaseProgression, _) => Update(PrereleaseProgression),
    }
}

/// 稳定通道只放行正式版
fn channel_blocks(channel: UpdateChannel, latest: &Version) -> bool {
    channel == UpdateChannel::Stable && is_prerelease(latest)
}

/// 只允许升级，不允许降级
fn is_newer(current: &Version, latest: &Version) -> bool {
    latest > current
}

/// 预发布 -> 稳定版：必须放行，否则用户永远卡在预发布
fn escape_to_stable(current: &Version, latest: &Version) -> bool {
    is_prerelease(current) && !is_prerelease(latest)
}

/// 预发布 -> 预发布：跟随通道，门槛不适用
fn prerelease_progression(current: &Version, latest: &Version) -> bool {
    is_prerelease(current) && is_prerelease(latest)
}

fn is_prerelease(version: &Version) -> bool {
    !version.pre.is_empty()
}

/// 数字幅度：只比主/次/补丁号，预发布与 build 标识不参与分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumericChange {
    Major,
    Minor,
    Patch,
    /// 三项数字全同，仅 build metadata 不同
    Identical,
}

impl NumericChange {
    /// 与 `Patch` 同档时，build metadata 发生变化则记为一次补丁级更新
    fn rank(self) -> u8 {
        match self {
            NumericChange::Identical | NumericChange::Patch => 1,
            NumericChange::Minor => 2,
            NumericChange::Major => 3,
        }
    }
}

fn numeric_change(current: &Version, latest: &Version) -> NumericChange {
    if latest.major > current.major {
        NumericChange::Major
    } else if latest.minor > current.minor {
        NumericChange::Minor
    } else if latest.patch > current.patch {
        NumericChange::Patch
    } else {
        NumericChange::Identical
    }
}

/// floor 语义，变化幅度 ≥ 门槛
fn meets_floor(change: NumericChange, floor: UpdateLevel) -> bool {
    let floor = match floor {
        UpdateLevel::Patch => 1,
        UpdateLevel::Minor => 2,
        UpdateLevel::Major => 3,
        // `Never` 已由上游 `Disabled` 挡下，此处只为穷尽
        UpdateLevel::Never => return false,
    };
    change.rank() >= floor
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::updater::model::UpdateChannel::{Prerelease, Stable};
    use crate::updater::model::UpdateLevel::{Major, Minor, Never, Patch};
    use SkipReason::*;
    use UpdateDecision::{Skip, Update};
    use UpdateReason::{Exempt, Force, Newer, PrereleaseEscape, PrereleaseProgression};

    fn v(s: &str) -> Version {
        Version::parse(s).expect("版本号应合法")
    }

    /// 版本层判定（severity 固定 normal）
    fn base(current: &str, latest: &str, level: UpdateLevel, channel: UpdateChannel) -> UpdateDecision {
        decide(&v(current), &v(latest), &Policy { level, channel }, Severity::Normal)
    }

    /// 完整判定（携带 severity）
    fn full(current: &str, latest: &str, level: UpdateLevel, channel: UpdateChannel, severity: Severity) -> UpdateDecision {
        decide(&v(current), &v(latest), &Policy { level, channel }, severity)
    }

    /// 预发布形态的三条谓词
    #[test]
    fn prerelease_predicates() {
        let shapes = [
            // current, latest, 逃逸, 递进
            ("1.2.0-rc.1", "1.2.0", true, false),
            ("1.2.0-rc.1", "1.3.0", true, false),
            ("1.2.0-rc.1", "1.2.0-rc.2", false, true),
            ("1.2.0", "1.3.0-rc.1", false, false),
            ("1.2.0", "1.3.0", false, false),
            ("1.2.0+b1", "1.2.0", false, false),
        ];
        for (current, latest, escape, progression) in shapes {
            let (current, latest) = (v(current), v(latest));
            assert_eq!(escape_to_stable(&current, &latest), escape, "{current} -> {latest}");
            assert_eq!(
                prerelease_progression(&current, &latest),
                progression,
                "{current} -> {latest}"
            );
        }

        assert!(channel_blocks(Stable, &v("1.3.0-rc.1")));
        assert!(!channel_blocks(Prerelease, &v("1.3.0-rc.1")));
        assert!(!channel_blocks(Stable, &v("1.3.0")));
    }

    /// 幅度分类与 floor 表
    #[test]
    fn numeric_change_and_floor() {
        assert_eq!(numeric_change(&v("1.2.1"), &v("2.0.0")), NumericChange::Major);
        assert_eq!(numeric_change(&v("1.2.1"), &v("1.3.0")), NumericChange::Minor);
        assert_eq!(numeric_change(&v("1.2.1"), &v("1.2.2")), NumericChange::Patch);
        assert_eq!(numeric_change(&v("1.2.3+abc"), &v("1.2.3+bcd")), NumericChange::Identical);

        let table = [
            (NumericChange::Major, [true, true, true]),
            (NumericChange::Minor, [true, true, false]),
            (NumericChange::Patch, [true, false, false]),
            (NumericChange::Identical, [true, false, false]),
        ];
        for (change, row) in table {
            for (floor, expected) in [Patch, Minor, Major].into_iter().zip(row) {
                assert_eq!(meets_floor(change, floor), expected, "{change:?} vs {floor:?}");
            }
            assert!(!meets_floor(change, Never), "{change:?} vs Never");
        }
    }

    /// 用户关闭更新，前向的稳定版一律 `Disabled`
    #[test]
    fn disabled_blocks_forward_updates() {
        for channel in [Stable, Prerelease] {
            for (current, latest) in [
                ("1.2.1", "2.0.0"),
                ("1.2.1", "1.3.0"),
                ("1.2.1", "1.2.2"),
                ("1.2.0-rc.1", "1.2.0"),
            ] {
                assert_eq!(base(current, latest, Never, channel), Skip(Disabled), "{current} -> {latest}");
            }
        }
    }

    /// 硬门禁优先于「用户关闭更新」
    #[test]
    fn hard_gates_are_never_penetrated() {
        for severity in [Severity::Important, Severity::Critical] {
            // 稳定通道 + 预发布
            assert_eq!(
                full("1.2.0", "1.3.0-rc.1", Never, Stable, severity),
                Skip(ChannelBlocked),
                "{severity:?} 不应把预发布推给稳定通道"
            );
            // 降级与同版本
            assert_eq!(full("2.0.0", "1.9.0", Never, Stable, severity), Skip(NotNewer));
            assert_eq!(full("1.2.3", "1.2.3", Never, Stable, severity), Skip(NotNewer));
        }
    }

    /// 稳定通道下 latest 是预发布 -> 一律拦截，与门槛无关
    #[test]
    fn stable_channel_blocks_prerelease() {
        for level in [Patch, Minor, Major] {
            for (current, latest) in [("1.2.1", "1.3.0-rc.1"), ("1.2.0-rc.1", "1.2.0-rc.2"), ("1.2.0", "1.2.0-rc.1")] {
                assert_eq!(
                    base(current, latest, level, Stable),
                    Skip(ChannelBlocked),
                    "{current} -> {latest}"
                );
            }
        }
    }

    /// 同版本与降级，任何通道、任何门槛均跳过
    #[test]
    fn equal_or_older_is_skipped() {
        for channel in [Stable, Prerelease] {
            for level in [Patch, Minor, Major] {
                assert_eq!(
                    base("1.2.3", "1.2.3", level, channel),
                    Skip(NotNewer),
                    "{channel:?} / {level:?}"
                );
                assert_eq!(
                    base("2.0.0", "1.9.9", level, channel),
                    Skip(NotNewer),
                    "{channel:?} / {level:?}"
                );
            }
        }
    }

    /// 预发布的两条路径绕过门槛，且原因可区分
    #[test]
    fn prerelease_paths_bypass_floor() {
        for level in [Patch, Minor, Major] {
            for channel in [Prerelease, Stable] {
                for (current, latest) in [("1.2.0-rc.1", "1.2.0"), ("1.2.0-rc.1", "1.3.0"), ("1.2.0-rc.1", "2.0.0")] {
                    assert_eq!(
                        base(current, latest, level, channel),
                        Update(PrereleaseEscape),
                        "{current} -> {latest}"
                    );
                }
            }
            for (current, latest) in [
                ("1.2.0-rc.1", "1.2.0-rc.2"),
                ("1.2.0-rc.1", "1.2.1-rc.1"),
                ("1.2.0-rc.1", "1.3.0-rc.1"),
                ("1.2.0-alpha.2", "1.2.0-alpha.10"),
            ] {
                assert_eq!(
                    base(current, latest, level, Prerelease),
                    Update(PrereleaseProgression),
                    "{current} -> {latest}"
                );
            }
        }
    }

    /// floor 全量表，3 种门槛 × 3 种数字变化
    #[test]
    fn floor_matrix_stable_to_stable() {
        for (latest, expected) in [
            ("1.2.2", [Update(Newer), Skip(BelowFloor), Skip(BelowFloor)]),
            ("1.3.0", [Update(Newer), Update(Newer), Skip(BelowFloor)]),
            ("2.0.0", [Update(Newer), Update(Newer), Update(Newer)]),
        ] {
            for (level, expected) in [Patch, Minor, Major].into_iter().zip(expected) {
                assert_eq!(
                    base("1.2.1", latest, level, Stable),
                    expected,
                    "1.2.1 -> {latest} / {level:?}"
                );
            }
        }
    }

    /// 通道切换（稳定 -> 预发布）：Prerelease 通道下仍按门槛
    #[test]
    fn channel_switch_stable_to_prerelease_uses_floor() {
        for (latest, expected) in [
            ("1.2.2-rc.1", [Update(Newer), Skip(BelowFloor), Skip(BelowFloor)]),
            ("1.3.0-rc.1", [Update(Newer), Update(Newer), Skip(BelowFloor)]),
            ("2.0.0-rc.1", [Update(Newer), Update(Newer), Update(Newer)]),
        ] {
            for (level, expected) in [Patch, Minor, Major].into_iter().zip(expected) {
                assert_eq!(
                    base("1.2.1", latest, level, Prerelease),
                    expected,
                    "1.2.1 -> {latest} / {level:?}"
                );
            }
        }
    }

    /// build metadata 是最后决胜键：`1.2.3+abc -> 1.2.3+bcd` 算补丁级
    #[test]
    fn build_metadata_is_patch_level() {
        assert_eq!(base("1.2.3+abc", "1.2.3+bcd", Patch, Stable), Update(Newer));
        assert_eq!(base("1.2.3+abc", "1.2.3+bcd", Minor, Stable), Skip(BelowFloor));
        assert_eq!(base("1.2.3+abc", "1.2.4", Patch, Stable), Update(Newer));
        assert_eq!(base("1.2.3+abc", "1.2.4", Minor, Stable), Skip(BelowFloor));
    }

    /// 融合矩阵：7 种基础判定 × 3 种 severity
    #[test]
    fn fuse_matrix() {
        let rows: &[(Verdict, [UpdateDecision; 3])] = &[
            (
                Verdict::Blocked(ChannelBlocked),
                [Skip(ChannelBlocked), Skip(ChannelBlocked), Skip(ChannelBlocked)],
            ),
            (Verdict::Blocked(NotNewer), [Skip(NotNewer), Skip(NotNewer), Skip(NotNewer)]),
            (Verdict::Blocked(Disabled), [Skip(Disabled), Skip(Disabled), Update(Force)]),
            (Verdict::BelowFloor, [Skip(BelowFloor), Update(Exempt), Update(Exempt)]),
            (Verdict::FloorMet, [Update(Newer), Update(Newer), Update(Newer)]),
            (Verdict::PrereleaseEscape, [Update(PrereleaseEscape); 3]),
            (Verdict::PrereleaseProgression, [Update(PrereleaseProgression); 3]),
        ];
        for (verdict, row) in rows {
            for (severity, want) in [Severity::Normal, Severity::Important, Severity::Critical]
                .into_iter()
                .zip(row)
            {
                assert_eq!(fuse(*verdict, severity), *want, "{verdict:?} × {severity:?}");
            }
        }
    }

    /// 不变量：任何组合下放行都意味着「严格更新」且未被通道拦截
    #[test]
    fn never_accepts_a_non_forward_or_hidden_version() {
        let versions = [
            "0.9.0",
            "1.0.0",
            "1.0.1",
            "1.1.0",
            "2.0.0",
            "1.0.0-rc.1",
            "1.1.0-rc.2",
            "2.0.0-alpha.1",
        ];
        for current in versions {
            for latest in versions {
                for level in [Patch, Minor, Major, Never] {
                    for channel in [Stable, Prerelease] {
                        for severity in [Severity::Normal, Severity::Important, Severity::Critical] {
                            let decision = full(current, latest, level, channel, severity);
                            if !decision.is_update() {
                                continue;
                            }
                            assert!(
                                v(latest) > v(current),
                                "{current} -> {latest} 被放行（{decision:?}），但目标版本并不更高"
                            );
                            assert!(
                                channel == Prerelease || !is_prerelease(&v(latest)),
                                "{current} -> {latest} 被放行（{decision:?}），但稳定通道不该看到预发布"
                            );
                        }
                    }
                }
            }
        }
    }
}
