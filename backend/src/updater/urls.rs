//! 更新源的链接构造：源 + 版本 → URL
//!
//!
//! | 本模块                | Python               | 形状                                                  |
//! |-----------------------|----------------------|-------------------------------------------------------|
//! | [`asset_url`]         | `asset_url`          | `<base>/releases/download/v<ver>/<name>`              |
//! | [`versions_index_url`]| `versions_index_url` | `<base>/releases/latest/download/versions.json`       |
//! | [`manifest_name`]     | `manifest_name`      | `latest-<slug>.json`                                  |
//! | [`manifest_url`]      | `manifest_url`       | `<base>/releases/download/v<ver>/latest-<slug>.json`  |

use crate::updater::model::UpdateSource;
use semver::Version;

/// GitHub 仓库页
const GITHUB_REPO: &str = "https://github.com/bytecascade-Lee/rollcaller";
/// CNB 仓库页，结尾的 `/-` 是 Release 路径的必需前缀
const CNB_REPO: &str = "https://cnb.cool/ordinary-glory/rollcaller/-";
/// 本地联调源，由 `scripts/release/serve.py` 起在同构路径上
const DEVELOP_REPO: &str = "http://localhost:14652/rollcaller";
/// 更新器仓库
const UPDATER_REPO_CNB: &str = "https://cnb.cool/ordinary-glory/updater/-";

/// 源的仓库基址（三类 URL 的公共前缀）
fn base(source: UpdateSource) -> &'static str {
    match source {
        UpdateSource::GitHub => GITHUB_REPO,
        UpdateSource::CNB => CNB_REPO,
        UpdateSource::Local => DEVELOP_REPO,
    }
}

/// 某个版本的产物直链：`<base>/releases/download/v<ver>/<name>`
pub fn asset_url(source: UpdateSource, version: &Version, file_name: &str) -> String {
    format!("{}/releases/download/v{version}/{file_name}", base(source))
}

/// 版本索引直链：`<base>/releases/latest/download/versions.json`
pub fn versions_index_url(source: UpdateSource) -> String {
    format!("{}/releases/latest/download/versions.json", base(source))
}

/// 清单文件名：`latest-github.json` / `latest-cnb.json` / `latest-develop.json`
pub fn manifest_name(source: UpdateSource) -> &'static str {
    match source {
        UpdateSource::GitHub => "latest-github.json",
        UpdateSource::CNB => "latest-cnb.json",
        UpdateSource::Local => "latest-develop.json",
    }
}

/// 某个版本的清单直链：`<base>/releases/download/v<ver>/latest-<slug>.json`
pub fn manifest_url(source: UpdateSource, version: &Version) -> String {
    asset_url(source, version, manifest_name(source))
}

/// 更新器清单直链：`<updater>/releases/download/v<ver>/latest-cnb.json`
pub fn updater_manifest_url(version: &str) -> String {
    format!("{UPDATER_REPO_CNB}/releases/download/v{version}/latest-cnb.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        text.parse().unwrap()
    }

    /// 三源的产物直链必须是**完整字面量**：形状一改，这里就必须跟着改
    #[test]
    fn asset_url_golden() {
        let name = "rollcaller-0.8.6-windows-x86_64-setup.exe";
        assert_eq!(
            asset_url(UpdateSource::GitHub, &v("0.8.6"), name),
            "https://github.com/bytecascade-Lee/rollcaller/releases/download/v0.8.6/rollcaller-0.8.6-windows-x86_64-setup.exe"
        );
        assert_eq!(
            asset_url(UpdateSource::CNB, &v("0.8.6"), name),
            "https://cnb.cool/ordinary-glory/rollcaller/-/releases/download/v0.8.6/rollcaller-0.8.6-windows-x86_64-setup.exe"
        );
        assert_eq!(
            asset_url(UpdateSource::Local, &v("0.8.6"), name),
            "http://localhost:14652/rollcaller/releases/download/v0.8.6/rollcaller-0.8.6-windows-x86_64-setup.exe"
        );
    }

    /// CNB 的 `/ -` 段：两条 Release 路径（产物 / 索引）都必须带，这是 0.8.4 事故的回归用例
    #[test]
    fn cnb_release_paths_carry_dash_segment() {
        assert!(asset_url(UpdateSource::CNB, &v("0.8.6"), "x.exe").contains("/-/releases/download/"));
        assert!(versions_index_url(UpdateSource::CNB).contains("/-/releases/latest/download/"));
        assert!(manifest_url(UpdateSource::CNB, &v("0.8.6")).contains("/-/releases/download/"));
        assert!(updater_manifest_url("0.3.0").contains("/-/releases/download/"));
        // GitHub 反而不能出现 `/-`
        assert!(!asset_url(UpdateSource::GitHub, &v("0.8.6"), "x.exe").contains("/-/"));
    }

    #[test]
    fn versions_index_url_golden() {
        assert_eq!(
            versions_index_url(UpdateSource::GitHub),
            "https://github.com/bytecascade-Lee/rollcaller/releases/latest/download/versions.json"
        );
        assert_eq!(
            versions_index_url(UpdateSource::CNB),
            "https://cnb.cool/ordinary-glory/rollcaller/-/releases/latest/download/versions.json"
        );
        assert_eq!(
            versions_index_url(UpdateSource::Local),
            "http://localhost:14652/rollcaller/releases/latest/download/versions.json"
        );
    }

    /// 清单名里的 `Local → develop` 不是笔误，与发布端 `Source.DEVELOP` 对齐
    #[test]
    fn manifest_name_uses_develop_for_local() {
        assert_eq!(manifest_name(UpdateSource::GitHub), "latest-github.json");
        assert_eq!(manifest_name(UpdateSource::CNB), "latest-cnb.json");
        assert_eq!(manifest_name(UpdateSource::Local), "latest-develop.json");
    }

    /// 清单名必须由 [`UpdateSource::slug`] 拼出：同一源的两种写法不允许漂移
    #[test]
    fn manifest_name_matches_slug() {
        for source in [UpdateSource::GitHub, UpdateSource::CNB, UpdateSource::Local] {
            assert_eq!(manifest_name(source), format!("latest-{}.json", source.slug()), "{source}");
        }
    }

    #[test]
    fn manifest_url_golden() {
        assert_eq!(
            manifest_url(UpdateSource::CNB, &v("0.8.6")),
            "https://cnb.cool/ordinary-glory/rollcaller/-/releases/download/v0.8.6/latest-cnb.json"
        );
        assert_eq!(
            manifest_url(UpdateSource::Local, &v("0.1.0+build.7")),
            "http://localhost:14652/rollcaller/releases/download/v0.1.0+build.7/latest-develop.json"
        );
    }

    #[test]
    fn updater_manifest_url_golden() {
        assert_eq!(
            updater_manifest_url("0.3.0"),
            "https://cnb.cool/ordinary-glory/updater/-/releases/download/v0.3.0/latest-cnb.json"
        );
    }

    /// 所有构造出来的串都必须是合法的 URL
    #[test]
    fn all_urls_parse() {
        let sources = [UpdateSource::GitHub, UpdateSource::CNB, UpdateSource::Local];
        for source in sources {
            for url in [
                asset_url(source, &v("0.8.6"), "x.exe"),
                versions_index_url(source),
                manifest_url(source, &v("0.8.6")),
            ] {
                url::Url::parse(&url).unwrap_or_else(|error| panic!("非法 URL {url}: {error}"));
            }
        }
        url::Url::parse(&updater_manifest_url("0.3.0")).expect("更新器清单 URL 非法");
    }
}
