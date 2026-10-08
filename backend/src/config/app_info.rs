use crate::common::{constant, enums};
use semver::Version;
use serde::Serialize;
use std::sync::LazyLock;
use ts_rs::TS;

#[cfg(debug_assertions)]
use serde::Deserialize;

#[derive(Clone, Debug, Serialize, Eq, Hash, PartialEq, TS)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
#[ts(export)]
#[cfg_attr(debug_assertions, serde(default))]
pub struct AppInfo {
    #[ts(type = "string")]
    pub version: Version,

    pub os: enums::sys::OS,
    pub arch: enums::sys::Arch,
    pub branch: Option<String>,
    pub commit_count: Option<i32>,
    pub commit_hash: Option<String>,
    pub abbreviated_hash: Option<String>,

    #[cfg_attr(
        debug_assertions,
        serde(deserialize_with = "crate::util::serde_utils::deserialize_optional_timestamp_from_iso_8601")
    )]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_optional_timestamp_to_millisecond_i64")]
    #[ts(type = "number | null")]
    pub commit_time: Option<jiff::Timestamp>,

    #[cfg_attr(
        debug_assertions,
        serde(deserialize_with = "crate::util::serde_utils::deserialize_optional_timestamp_from_iso_8601")
    )]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_optional_timestamp_to_millisecond_i64")]
    #[ts(type = "number | null")]
    pub build_time: Option<jiff::Timestamp>,
}

impl Default for AppInfo {
    fn default() -> Self {
        Self {
            version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            os: constant::sys::OS,
            arch: constant::sys::ARCH,
            branch: None,
            commit_count: None,
            commit_hash: None,
            abbreviated_hash: None,
            commit_time: None,
            build_time: None,
        }
    }
}

#[cfg(debug_assertions)]
fn load() -> AppInfo {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("gen/build-info.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {} 失败：{e}；请先运行 codegen 脚本", path.display()));
    serde_json::from_str::<AppInfo>(&raw).unwrap_or_else(|e| panic!("解析 {} 失败：{e}", path.display()))
}

#[cfg(not(debug_assertions))]
fn load() -> AppInfo {
    mod gen {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/gen/build_info.rs"));
    }

    fn parse_ts(s: Option<&str>) -> Option<jiff::Timestamp> {
        s.and_then(|v| v.parse::<jiff::Timestamp>().ok())
    }

    AppInfo {
        version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
        os: constant::sys::OS,
        arch: constant::sys::ARCH,
        branch: gen::BRANCH.map(str::to_owned),
        commit_count: gen::COMMIT_COUNT,
        commit_hash: gen::COMMIT_HASH.map(str::to_owned),
        abbreviated_hash: gen::ABBREVIATED_HASH.map(str::to_owned),
        commit_time: parse_ts(gen::COMMIT_TIME),
        build_time: parse_ts(gen::BUILD_TIME),
    }
}

static APP_INFO: LazyLock<AppInfo> = LazyLock::new(load);

/// 获取 AppInfo 的引用
pub fn app_info() -> &'static AppInfo {
    &APP_INFO
}
