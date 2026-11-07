//! 更新源的链接构造：源 + 版本 → URL
//!
//! # 唯一性
//!
//! 本模块是 Rust 侧**唯一**知道 Release 路径形状的地方，与发布端 `scripts/release/urls.py`
//! 逐函数对应（两边各一份实现，靠 golden 测试与发布后自检防漂移，不做代码生成）：
//!
//! | 本模块                | Python               | 形状                                                  |
//! |-----------------------|----------------------|-------------------------------------------------------|
//! | [`asset_url`]         | `asset_url`          | `<base>/releases/download/v<ver>/<name>`              |
//! | [`versions_index_url`]| `versions_index_url` | `<base>/releases/latest/download/versions.json`       |
//! | [`manifest_name`]     | `manifest_name`      | `latest-<slug>.json`                                  |
//! | [`manifest_url`]      | `manifest_url`       | `<base>/releases/download/v<ver>/latest-<slug>.json`  |
//!
//! 唯一的单向差异是 [`updater_manifest_url`]：更新器在另一个仓库发布，Python 侧没有对应物。
//!
//! # CNB 的 `/-/`
//!
//! CNB 的 Release 直链必须带 `/-/` 段（`cnb.cool/<owner>/<repo>/-/releases/...`），漏掉就是
//! HTTP 404 —— 0.8.4 / 0.8.5 两次发布的清单都因此错发过（产物本身在 `/-/` 下是 200）。
//! 所以 `/-` 只写在 [`CNB_REPO`] 一处，其余路径全部经 [`base`] 拼装，不要再手写完整链接。
//!
//! # 版本段
//!
//! 一律 `v<语义化版本>`（[`Version`] 的 Display），与 Python 的 `Version.to_tag()` 逐字符一致。
//! Develop 形态的版本自带 `+build` 元数据，也原样进 URL——本地联调服务按同一串比对。

use crate::updater::model::UpdateSource;
use semver::Version;

/// GitHub 仓库页（Release 直链的基址，无结尾 `/`）
const GITHUB_REPO: &str = "https://github.com/bytecascade-Lee/rollcaller";
/// CNB 仓库页：结尾的 `/-` 是 Release 路径的必需前缀，**不要删**
const CNB_REPO: &str = "https://cnb.cool/ordinary-glory/rollcaller/-";
/// 本地联调源，由 `scripts/release/serve.py` 起在同构路径上
const DEVELOP_REPO: &str = "http://localhost:14652/rollcaller";
/// 更新器仓库（与主仓库不同，故单独一条；同样需要 `/-`）
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
///
/// `file_name` 同时是客户端落盘的产物名（`Artifact::file_name()` 取 url 最后一段），
/// 所以发布侧必须让它自带版本 / 平台 / 架构。
///
/// # 例
/// `asset_url(CNB, 0.8.6, "rollcaller-0.8.6-windows-x86_64-setup.exe")` →
/// `https://cnb.cool/ordinary-glory/rollcaller/-/releases/download/v0.8.6/rollcaller-0.8.6-windows-x86_64-setup.exe`
pub fn asset_url(source: UpdateSource, version: &Version, file_name: &str) -> String {
    format!("{}/releases/download/v{version}/{file_name}", base(source))
}

/// 版本索引直链：`<base>/releases/latest/download/versions.json`
///
/// 恒指向"当前 latest 的 Release"里的那一份，因此每次检查都要拉、不缓存。
pub fn versions_index_url(source: UpdateSource) -> String {
    format!("{}/releases/latest/download/versions.json", base(source))
}

/// 清单文件名：`latest-github.json` / `latest-cnb.json` / `latest-develop.json`
///
/// 注意 `Local` 源的文件名是 `develop` 而不是 `local`——发布端 `Source.DEVELOP` 的取值即
/// `develop`，两端必须逐字符一致。
pub fn manifest_name(source: UpdateSource) -> &'static str {
    match source {
        UpdateSource::GitHub => "latest-github.json",
        UpdateSource::CNB => "latest-cnb.json",
        UpdateSource::Local => "latest-develop.json",
    }
}

/// 某个版本的清单直链：`<base>/releases/download/v<ver>/latest-<slug>.json`
///
/// 清单按版本寻址（不是 `latest/download`），因为它是"这一版发布了什么"的凭据。
pub fn manifest_url(source: UpdateSource, version: &Version) -> String {
    asset_url(source, version, manifest_name(source))
}

/// 更新器清单直链（锁定版本）：`<updater>/releases/download/v<ver>/latest-cnb.json`
///
/// 更新器只有 CNB 一个源，且按 **tag** 取（不用 latest）——更新器侧解释我们写出的
/// `config`，旧版会直接拒绝，故版本必须由调用方显式传入（通常传
/// [`crate::updater::constant::UPDATER_VERSION`]）。
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

    /// 所有构造出来的串都必须是合法 URL（防手滑写错分隔符）
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
