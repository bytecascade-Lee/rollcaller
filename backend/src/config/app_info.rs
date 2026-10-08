use crate::common::{constant, enums};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq, TS)]
#[ts(export)]
#[serde(default)]
pub struct AppInfo {
    #[ts(type = "string")]
    pub version: Version,

    pub os: enums::sys::OS,
    pub arch: enums::sys::Arch,
    pub branch: Option<String>,
    pub commit_count: Option<i32>,
    pub commit_hash: Option<String>,
    pub abbreviated_hash: Option<String>,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_optional_timestamp_from_iso_8601")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_optional_timestamp_to_millisecond_i64")]
    #[ts(type = "number | null")]
    pub commit_time: Option<jiff::Timestamp>,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_optional_timestamp_from_iso_8601")]
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

static APP_INFO: LazyLock<AppInfo> = LazyLock::new(|| {
    #[cfg(debug_assertions)]
    {
        serde_json::from_str::<AppInfo>(
            &fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("gen/build-info.json")).unwrap(),
        )
        .unwrap()
    }
    #[cfg(not(debug_assertions))]
    {
        serde_json::from_str::<AppInfo>(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/gen/build_info.txt"))).unwrap()
    }
});

/// 获取 AppInfo 的引用
pub fn app_info() -> &'static AppInfo {
    &APP_INFO
}
