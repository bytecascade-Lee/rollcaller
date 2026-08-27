use serde::{Deserialize, Serialize};
use std::env;
use std::string::ToString;
use std::sync::LazyLock;
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq, TS)]
#[ts(export)]
pub struct AppInfo {
    pub os: String,
    pub arch: String,
    pub branch: String,
    pub commit_count: String,
    pub short_hash: String,
    pub commit_time: String,
    pub version: String,
    pub build_time: String,
}

static APP_INFO: LazyLock<AppInfo> = LazyLock::new(|| AppInfo {
    os: env::consts::OS.to_string(),
    arch: (if env::consts::ARCH == "aarch64" { "arm64" } else { "x86_64" }).to_string(),
    branch: env!("GIT_BRANCH").to_string(),
    commit_count: env!("GIT_COMMIT_COUNT").to_string(),
    short_hash: env!("GIT_SHORT_HASH").to_string(),
    commit_time: env!("GIT_COMMIT_TIME").to_string(),
    version: env!("CARGO_PKG_VERSION").to_string(),
    build_time: env!("BUILD_TIME").to_string(),
});

/// 获取 AppInfo 的引用
pub fn app_info() -> &'static AppInfo {
    &APP_INFO
}
