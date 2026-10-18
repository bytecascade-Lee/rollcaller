use crate::common::{constant, enums};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, TS)]
pub struct BuildInfo {
    pub ref_name: String,
    pub commit_count: i32,
    pub commit_hash: String,
    pub abbreviated_hash: String,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_timestamp_from_iso_8601")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_timestamp_to_millisecond_i64")]
    #[ts(type = "number")]
    pub commit_time: jiff::Timestamp,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_timestamp_from_iso_8601")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_timestamp_to_millisecond_i64")]
    #[ts(type = "number")]
    pub build_time: jiff::Timestamp,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct AppInfo {
    #[ts(type = "string")]
    pub version: Version,

    pub os: enums::sys::OS,
    pub arch: enums::sys::Arch,

    #[serde(flatten)]
    pub build_info: BuildInfo,
}

#[cfg(debug_assertions)]
fn raw() -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("gen/build/build_info.json");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {} 失败：{e}；请先运行 codegen 脚本", path.display()))
}

#[cfg(not(debug_assertions))]
fn raw() -> &'static str {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/gen/build/build_info.json"))
}

fn load() -> AppInfo {
    let build_info: BuildInfo = serde_json::from_str(&raw())
        .unwrap_or_else(|e| panic!("解析构建元数据失败：{e}；build_info.json 与 BuildInfo 不一致，请重新运行 codegen 脚本"));

    AppInfo {
        version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
        os: constant::sys::OS,
        arch: constant::sys::ARCH,
        build_info,
    }
}

static APP_INFO: LazyLock<AppInfo> = LazyLock::new(load);

/// 获取 AppInfo 的引用
pub fn app_info() -> &'static AppInfo {
    &APP_INFO
}
