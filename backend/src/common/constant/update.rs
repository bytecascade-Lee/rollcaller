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

// ---------------------------------------------------------------------------
// 本地联调（dev 通道）
//
// 通过环境变量 ROLLCALLER_UPDATE_BASE（如 http://127.0.0.1:14652）把更新链路
// 的数据源切到本地 HTTP 服务，URL 形状与远端 GitHub/CNB Release 资产同构
// （/releases/latest/download/versions.json、/releases/download/v<ver>/latest-dev.json），
// 便于未来"特殊渠道"直接复用同一套拼 URL 逻辑。生产环境不设置该变量，零影响。
//
// 本地清单文件名的 latest-{source} 命名延续远端约定：远端是 latest-github/cnb，
// 本地是 latest-dev；manifest 缓存子目录也因此用 "dev"（cache/update/dev/<ver>.json），
// 与 github/cnb 区分，避免本地联调污染真实缓存。
// ---------------------------------------------------------------------------

/// 本地联调（dev 通道）清单文件名
pub const LATEST_MANIFEST_LOCAL: &str = "latest-dev.json";
/// 本地联调清单缓存的子目录名（`cache_dir/update/dev/`）
pub const LOCAL_CACHE_SUBDIR: &str = "dev";
/// 本地数据源环境变量：http(s) base，如 `http://127.0.0.1:14652`
pub const UPDATE_BASE_ENV: &str = "ROLLCALLER_UPDATE_BASE";
/// 本地更新形态环境变量：`install` | `portable`，覆盖真实运行形态（仅更新链路）
pub const UPDATE_MODE_ENV: &str = "ROLLCALLER_UPDATE_MODE";

pub const GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller";
pub const CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller";

pub const GITHUB_PORTABLE_UPDATER: &str = "https://github.com/bytecascade-Lee/updater";
pub const CNB_PORTABLE_UPDATER: &str = "https://cnb.cool/ordinary-glory/updater";

pub const RELEASE_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/tag/v__VERSION__/";
pub const RELEASE_CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller/-/releases/tag/v__VERSION__/";

pub const LATEST_MANIFEST_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/latest/download/latest-github.json";
pub const LATEST_MANIFEST_CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller/-/releases/latest/download/latest-cnb.json";

pub const VERSIONS_INDEX_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/latest/download/versions.json";
pub const VERSIONS_INDEX_CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller/-/releases/latest/download/versions.json";

pub const SPECIFIED_LATEST_MANIFEST_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/download/v__VERSION__/latest-github.json";
pub const SPECIFIED_LATEST_MANIFEST_CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller/-/releases/download/v__VERSION__/latest-cnb.json";

pub const PORTABLE_UPDATER_GITHUB: &str = "https://github.com/bytecascade-Lee/updater";
pub const PORTABLE_UPDATER_CNB: &str = "https://cnb.cool/ordinary-glory/updater";

pub const PORTABLE_UPDATE_RELEASE_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/tag/v__VERSION__/";
pub const PORTABLE_UPDATE_RELEASE_CNB: &str = "https://cnb.cool/ordinary-glory/updater/-/releases/tag/v__VERSION__/";

pub const SPECIFIED_PORTABLE_UPDATE_RELEASE_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/tag/v__VERSION__/latest-github.json";
pub const SPECIFIED_PORTABLE_UPDATE_RELEASE_CNB: &str = "https://cnb.cool/ordinary-glory/updater/-/releases/tag/v__VERSION__/latest-cnb.json";

pub const PORTABLE_UPDATER_LATEST_MANIFEST_GITHUB: &str = "https://github.com/bytecascade-Lee/updater/-/releases/latest/download/latest-github.json";
pub const PORTABLE_UPDATER_LATEST_MANIFEST_CNB: &str = "https://cnb.cool/ordinary-glory/updater/-/releases/latest/download/latest-cnb.json";

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
            LATEST_MANIFEST_GITHUB,
            LATEST_MANIFEST_CNB,
            VERSIONS_INDEX_GITHUB,
            VERSIONS_INDEX_CNB,
            SPECIFIED_LATEST_MANIFEST_GITHUB,
            SPECIFIED_LATEST_MANIFEST_CNB,
            PORTABLE_UPDATER_GITHUB,
            PORTABLE_UPDATER_CNB,
            PORTABLE_UPDATER_LATEST_MANIFEST_GITHUB,
            PORTABLE_UPDATER_LATEST_MANIFEST_CNB,
        ];

        for url_str in urls {
            Url::parse(url_str).expect(&format!("无效的 URL 常量: {}", url_str));
        }
    }
}
