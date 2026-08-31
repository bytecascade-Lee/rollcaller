//! 版本更新决策（纯函数，输入输出均为普通数据，便于单元测试，不依赖 Tauri）

use semver::Version;
use std::cmp::Ordering;

/// 更新决策结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateDecision {
    /// 不更新
    Skip,
    /// 更新
    Update,
}

/// 判断是否应该更新
///
/// 规则（现阶段从简，预留扩展点）：
/// - `latest > current` → `Update`
/// - `latest == current` → `Skip`
/// - `latest < current` → `allow_downgrade ? Update : Skip`
/// - `latest` 是 pre-release（如 `1.3.0-beta.1`）→ 仅当 `current` 也是 pre-release 才 `Update`，否则 `Skip`
/// - `current` 是 pre-release 且 `latest > current` → `Update`
pub fn decide(current: &Version, latest: &Version, allow_downgrade: bool) -> UpdateDecision {
    // pre-release 门禁：稳定版 current 不接受 pre-release latest（即使允许降级）
    if !latest.pre.is_empty() && current.pre.is_empty() {
        return UpdateDecision::Skip;
    }
    match latest.cmp(current) {
        Ordering::Greater => UpdateDecision::Update,
        Ordering::Equal => UpdateDecision::Skip,
        Ordering::Less => {
            if allow_downgrade {
                UpdateDecision::Update
            } else {
                UpdateDecision::Skip
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn test_upgrade() {
        assert_eq!(decide(&v("1.0.0"), &v("1.1.0"), false), UpdateDecision::Update);
        assert_eq!(decide(&v("1.1.0"), &v("1.1.1"), false), UpdateDecision::Update);
        // 跨大版本升级
        assert_eq!(decide(&v("1.9.9"), &v("2.0.0"), false), UpdateDecision::Update);
    }

    #[test]
    fn test_same_version() {
        assert_eq!(decide(&v("1.1.0"), &v("1.1.0"), false), UpdateDecision::Skip);
        assert_eq!(decide(&v("1.1.0"), &v("1.1.0"), true), UpdateDecision::Skip);
    }

    #[test]
    fn test_downgrade() {
        assert_eq!(decide(&v("1.2.0"), &v("1.1.0"), false), UpdateDecision::Skip);
        assert_eq!(decide(&v("1.2.0"), &v("1.1.0"), true), UpdateDecision::Update);
    }

    #[test]
    fn test_prerelease_latest_blocked_for_stable_current() {
        // 稳定版 current 不接受 pre-release latest（即使允许降级）
        assert_eq!(
            decide(&v("1.3.0"), &v("1.3.0-beta.1"), false),
            UpdateDecision::Skip
        );
        assert_eq!(
            decide(&v("1.3.0"), &v("1.3.0-beta.1"), true),
            UpdateDecision::Skip
        );
        assert_eq!(
            decide(&v("1.2.0"), &v("1.3.0-beta.1"), false),
            UpdateDecision::Skip
        );
    }

    #[test]
    fn test_prerelease_to_prerelease() {
        assert_eq!(
            decide(&v("1.3.0-beta.1"), &v("1.3.0-beta.2"), false),
            UpdateDecision::Update
        );
        assert_eq!(
            decide(&v("1.3.0-beta.2"), &v("1.3.0-beta.1"), false),
            UpdateDecision::Skip
        );
        assert_eq!(
            decide(&v("1.3.0-beta.2"), &v("1.3.0-beta.1"), true),
            UpdateDecision::Update
        );
        assert_eq!(
            decide(&v("1.3.0-beta.1"), &v("1.3.0-beta.1"), false),
            UpdateDecision::Skip
        );
    }

    #[test]
    fn test_prerelease_current_to_stable() {
        // current 是 pre-release 且 latest > current → Update
        assert_eq!(
            decide(&v("1.2.0-beta.1"), &v("1.2.0"), false),
            UpdateDecision::Update
        );
        assert_eq!(
            decide(&v("1.3.0-beta.1"), &v("1.3.0"), false),
            UpdateDecision::Update
        );
        assert_eq!(
            decide(&v("1.3.0-beta.1"), &v("1.4.0"), false),
            UpdateDecision::Update
        );
    }
}
