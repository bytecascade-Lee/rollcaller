use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq, TS)]
#[ts(export)]
pub struct AppInfo {
    pub branch: String,
    pub commit_count: String,
    pub short_hash: String,
    pub commit_time: String,
    pub version: String,
    pub build_time: String,
}

impl AppInfo {
    pub fn new() -> Self {
        AppInfo {
            branch: env!("GIT_BRANCH").to_string(),
            commit_count: env!("GIT_COMMIT_COUNT").to_string(),
            short_hash: env!("GIT_SHORT_HASH").to_string(),
            commit_time: env!("GIT_COMMIT_TIME").to_string(),
            version: env!("VERSION").to_string(),
            build_time: env!("BUILD_TIME").to_string(),
        }
    }
}
