use crate::updater::model::{UpdateChannel, UpdateLevel};

/// 默认更新等级策略：`level = Patch`
pub const DEFAULT_UPDATE_LEVEL: UpdateLevel = UpdateLevel::Patch;
/// 默认更新渠道策略：`channel = Stable`。
pub const DEFAULT_UPDATE_CHANNEL: UpdateChannel = UpdateChannel::Stable;

pub const LATEST_MANIFEST_START_GITHUB: &str = "0.8.0";
pub const LATEST_MANIFEST_START_CNB: &str = "0.8.0";

pub const VERSIONS_INDEX_START_GITHUB: &str = "0.8.0";
pub const VERSIONS_INDEX_START_CNB: &str = "0.8.0";

/// 便携版更新器的锁定版本
pub const UPDATER_VERSION: &str = "0.3.0";

pub const DEVELOP_UPDATE_BIN_NAME: &str = "rollcaller-update-from-develop.exe";

#[cfg(test)]
mod tests {
    use crate::updater::urls::{
        CNB, DEVELOP, GITHUB, LATEST_MANIFEST_CNB, LATEST_MANIFEST_DEVELOP, LATEST_MANIFEST_GITHUB, PORTABLE_UPDATER_CNB,
        PORTABLE_UPDATER_DEVELOP, PORTABLE_UPDATER_GITHUB, RELEASE_CNB, RELEASE_DEVELOP, RELEASE_GITHUB,
        SPECIFIED_LATEST_MANIFEST_CNB, SPECIFIED_LATEST_MANIFEST_DEVELOP, SPECIFIED_LATEST_MANIFEST_GITHUB,
        UPDATER_MANIFEST_CNB, VERSIONS_INDEX_CNB, VERSIONS_INDEX_DEVELOP, VERSIONS_INDEX_GITHUB,
    };
    use url::Url;

    #[test]
    fn validate_all_urls() {
        // 把它们放到一个数组里遍历，防止遗漏
        let urls = [
            GITHUB,
            CNB,
            DEVELOP,
            PORTABLE_UPDATER_GITHUB,
            PORTABLE_UPDATER_CNB,
            PORTABLE_UPDATER_DEVELOP,
            RELEASE_GITHUB,
            RELEASE_CNB,
            RELEASE_DEVELOP,
            LATEST_MANIFEST_GITHUB,
            LATEST_MANIFEST_CNB,
            LATEST_MANIFEST_DEVELOP,
            VERSIONS_INDEX_GITHUB,
            VERSIONS_INDEX_CNB,
            VERSIONS_INDEX_DEVELOP,
            SPECIFIED_LATEST_MANIFEST_GITHUB,
            SPECIFIED_LATEST_MANIFEST_CNB,
            SPECIFIED_LATEST_MANIFEST_DEVELOP,
            UPDATER_MANIFEST_CNB,
        ];

        for url_str in urls {
            Url::parse(url_str).unwrap_or_else(|error| panic!("无效的 URL 常量: {url_str}: {error}"));
        }
    }
}

pub const ROLLCALLER_UPDATE_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IERBNTVDNkMyRUQyMzUxQUQKUldTdFVTUHR3c1pWMnJ3ME55Mkl4d1BpRXpmdmtLOGo0Q3cxeGtrbVJPQjEyQWVyUG5rbGZLS3AK";
