use crate::common::enums::update::{UpdateChannel, UpdateLevel};

/// 默认更新等级策略：`level = Patch`
pub const DEFAULT_UPDATE_LEVEL: UpdateLevel = UpdateLevel::Patch;
/// 默认更新渠道策略：`channel = Stable`。
pub const DEFAULT_UPDATE_CHANNEL: UpdateChannel = UpdateChannel::Stable;

pub const LATEST_MANIFEST_START_GITHUB: &str = "0.8.0";
pub const LATEST_MANIFEST_START_CNB: &str = "0.8.0";

pub const VERSIONS_INDEX_START_GITHUB: &str = "0.8.0";
pub const VERSIONS_INDEX_START_CNB: &str = "0.8.0";

pub const PLACEHOLDER: &str = "__VERSION__";

pub const GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller";
pub const CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller";
pub const DEVELOP: &str = "http://localhost:14652/rollcaller";

pub const RELEASE_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/tag/v__VERSION__/";
pub const RELEASE_CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller/-/releases/tag/v__VERSION__/";
pub const RELEASE_DEVELOP: &str = "http://localhost:14652/rollcaller/releases/tag/v__VERSION__/";

pub const LATEST_MANIFEST_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/latest/download/latest-github.json";
pub const LATEST_MANIFEST_CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller/-/releases/latest/download/latest-cnb.json";
pub const LATEST_MANIFEST_DEVELOP: &str = "http://localhost:14652/rollcaller/releases/latest/download/latest-develop.json";

pub const VERSIONS_INDEX_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/latest/download/versions.json";
pub const VERSIONS_INDEX_CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller/-/releases/latest/download/versions.json";
pub const VERSIONS_INDEX_DEVELOP: &str = "http://localhost:14652/rollcaller/releases/latest/download/versions.json";

pub const SPECIFIED_LATEST_MANIFEST_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/download/v__VERSION__/latest-github.json";
pub const SPECIFIED_LATEST_MANIFEST_CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller/-/releases/download/v__VERSION__/latest-cnb.json";
pub const SPECIFIED_LATEST_MANIFEST_DEVELOP: &str = "http://localhost:14652/rollcaller/releases/download/v__VERSION__/latest-develop.json";

pub const PORTABLE_UPDATER_GITHUB: &str = "https://github.com/bytecascade-Lee/updater";
pub const PORTABLE_UPDATER_CNB: &str = "https://cnb.cool/ordinary-glory/updater";
pub const PORTABLE_UPDATER_DEVELOP: &str = "http://localhost:14652/updater";

// ── 便携版更新器（版本锁定）──────────────────────────────────────────────────────
//
// 每个 rollcaller 版本固定配对一个更新器版本：我们写出的 config（`version: 3`）由更新器侧
// 解释，旧更新器会直接拒绝（`config error: unsupported config version 3`，且失败发生在它的
// logger 初始化之前，日志一字不写）。故清单按 **tag** 取，不用 latest。
//
// 产物名 / sha256 / 字节数（以及将来可能的 minisign 签名）一律由清单给出，不写进常量：
// 二进制改名、加架构、加签名都只是清单内容的变化。
//
// 升级流程：只改 [`UPDATER_VERSION`]，并确认新版本支持的 config 版本 ≥
// `install/{portable,develop}.rs::compose_config` 写出的版本。

/// 便携版更新器的锁定版本
pub const UPDATER_VERSION: &str = "0.3.0";

/// 锁定版本的更新器清单（`__VERSION__` → [`UPDATER_VERSION`]）
pub const UPDATER_MANIFEST_CNB: &str =
    "https://cnb.cool/ordinary-glory/updater/-/releases/download/v__VERSION__/latest-cnb.json";

pub const DEVELOP_UPDATE_BIN_NAME: &str = "rollcaller-update-from-develop.exe";

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
            Url::parse(url_str).expect(&format!("无效的 URL 常量: {}", url_str));
        }
    }
}
