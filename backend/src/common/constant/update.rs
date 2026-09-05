use crate::common::enums::update::{UpdateChannel, UpdateLevel};

/// 默认更新等级策略：`level = Patch`
pub const DEFAULT_UPDATE_LEVEL: UpdateLevel = UpdateLevel::Patch;
/// 默认更新渠道策略：`channel = Stable`。
pub const DEFAULT_UPDATE_CHANNEL: UpdateChannel = UpdateChannel::Stable;

pub const LATEST_FILE_START_GITHUB: &str = "0.8.0";
pub const LATEST_FILE_START_CNB: &str = "0.8.0";

pub const VERSIONS_FILE_START_GITHUB: &str = "0.8.0";
pub const VERSIONS_FILE_START_CNB: &str = "0.8.0";

pub const GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller";
pub const CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller";

pub const GITHUB_PORTABLE_UPDATER: &str = "https://github.com/bytecascade-Lee/updater";
pub const CNB_PORTABLE_UPDATER: &str = "https://cnb.cool/ordinary-glory/updater";

pub const RELEASE_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/tag/v__VERSION__/";
pub const RELEASE_CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller/-/releases/tag/v__VERSION__/";

pub const LATEST_FILE_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/latest/download/latest-github.json";
pub const LATEST_FILE_CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller/-/releases/latest/download/latest-cnb.json";

pub const VERSIONS_FILE_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/latest/download/versions.json";
pub const VERSIONS_FILE_CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller/-/releases/latest/download/versions.json";

pub const PORTABLE_UPDATER_GITHUB: &str = "https://github.com/bytecascade-Lee/updater";
pub const PORTABLE_UPDATER_CNB: &str = "https://cnb.cool/ordinary-glory/updater";

pub const PORTABLE_UPDATE_RELEASE_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/tag/v__VERSION__/";
pub const PORTABLE_UPDATE_RELEASE_CNB: &str = "https://cnb.cool/ordinary-glory/updater/-/releases/tag/v__VERSION__/";

pub const PORTABLE_UPDATER_LATEST_FILE_GITHUB: &str = "https://github.com/bytecascade-Lee/updater/-/releases/latest/download/latest-github.json";
pub const PORTABLE_UPDATER_LATEST_FILE_CNB: &str = "https://cnb.cool/ordinary-glory/updater/-/releases/latest/download/latest-cnb.json";

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    #[test]
    fn validate_all_urls() {
        // 把它们放到一个数组里遍历，防止遗漏
        let urls = [
            GITHUB,
            CNB,
            GITHUB_PORTABLE_UPDATER,
            CNB_PORTABLE_UPDATER,
            RELEASE_GITHUB,
            RELEASE_CNB,
            LATEST_FILE_GITHUB,
            LATEST_FILE_CNB,
            VERSIONS_FILE_GITHUB,
            VERSIONS_FILE_CNB,
            PORTABLE_UPDATER_GITHUB,
            PORTABLE_UPDATER_CNB,
            PORTABLE_UPDATER_LATEST_FILE_GITHUB,
            PORTABLE_UPDATER_LATEST_FILE_CNB,
        ];

        for url_str in urls {
            Url::parse(url_str).expect(&format!("无效的 URL 常量: {}", url_str));
        }
    }
}
