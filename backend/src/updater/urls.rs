use crate::updater::constant::UPDATER_VERSION;
use crate::updater::model::UpdateSource;
use semver::Version;

pub const PLACEHOLDER: &str = "__VERSION__";
pub const GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller";
pub const CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller";
pub const DEVELOP: &str = "http://localhost:14652/rollcaller";
pub const RELEASE_GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/tag/v__VERSION__/";
pub const RELEASE_CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller/-/releases/tag/v__VERSION__/";
pub const RELEASE_DEVELOP: &str = "http://localhost:14652/rollcaller/releases/tag/v__VERSION__/";
pub const LATEST_MANIFEST_GITHUB: &str =
    "https://github.com/bytecascade-Lee/rollcaller/releases/latest/download/latest-github.json";
pub const LATEST_MANIFEST_CNB: &str =
    "https://cnb.cool/ordinary-glory/rollcaller/-/releases/latest/download/latest-cnb.json";
pub const LATEST_MANIFEST_DEVELOP: &str = "http://localhost:14652/rollcaller/releases/latest/download/latest-develop.json";
pub const VERSIONS_INDEX_GITHUB: &str =
    "https://github.com/bytecascade-Lee/rollcaller/releases/latest/download/versions.json";
pub const VERSIONS_INDEX_CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller/-/releases/latest/download/versions.json";
pub const VERSIONS_INDEX_DEVELOP: &str = "http://localhost:14652/rollcaller/releases/latest/download/versions.json";
pub const SPECIFIED_LATEST_MANIFEST_GITHUB: &str =
    "https://github.com/bytecascade-Lee/rollcaller/releases/download/v__VERSION__/latest-github.json";
pub const SPECIFIED_LATEST_MANIFEST_CNB: &str =
    "https://cnb.cool/ordinary-glory/rollcaller/-/releases/download/v__VERSION__/latest-cnb.json";
pub const SPECIFIED_LATEST_MANIFEST_DEVELOP: &str =
    "http://localhost:14652/rollcaller/releases/download/v__VERSION__/latest-develop.json";
pub const PORTABLE_UPDATER_GITHUB: &str = "https://github.com/bytecascade-Lee/updater";
pub const PORTABLE_UPDATER_CNB: &str = "https://cnb.cool/ordinary-glory/updater";
pub const PORTABLE_UPDATER_DEVELOP: &str = "http://localhost:14652/updater";
/// 锁定版本的更新器清单（`__VERSION__` → [`UPDATER_VERSION`]）
pub const UPDATER_MANIFEST_CNB: &str =
    "https://cnb.cool/ordinary-glory/updater/-/releases/download/v__VERSION__/latest-cnb.json";

fn base(source: &UpdateSource) -> &'static str {
    match source {
        &UpdateSource::GitHub => "https://github.com/bytecascade-Lee/rollcaller",
        &UpdateSource::CNB => "https://cnb.cool/ordinary-glory/rollcaller/-",
        &UpdateSource::Local => "http://localhost:14652/rollcaller",
    }
}

pub fn asset_url(source: &UpdateSource, version: &Version, file_name: &str) -> String {
    format!("{}/releases/download/{}/{file_name}", base(source), version.to_tag())
}

pub fn manifest_name(source: &UpdateSource) -> &'static str {
    match source {
        UpdateSource::GitHub => "latest-github.json",
        UpdateSource::CNB => "latest-cnb.json",
        UpdateSource::Local => "latest-develop.json",
    }
}

pub fn manifest_url(source: &UpdateSource, version: Version) -> String {
    asset_url(source, &version, manifest_name(source))
}
