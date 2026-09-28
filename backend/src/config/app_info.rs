use crate::common::{constant, enums};
use serde::{Deserialize, Serialize};
use std::string::ToString;
use std::sync::LazyLock;
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq, TS)]
#[ts(export)]
pub struct AppInfo {
    pub os: enums::sys::OS,
    pub arch: enums::sys::Arch,
    pub branch: String,
    pub commit_count: String,
    pub short_hash: String,
    pub commit_time: String,
    pub version: String,
    pub build_time: String,
}

/// 构建信息载体，由 `scripts/generate_build_info.py` 生成（`backend/build-info.json`）
///
/// 时间为 ISO 8601 且带本机时区偏移，例如 `2026-09-28T12:38:33+08:00`。
#[derive(Debug, Deserialize)]
struct BuildInfo {
    branch: String,
    commit_count: String,
    short_hash: String,
    commit_time: String,
    build_time: String,
}

impl Default for BuildInfo {
    /// 文件缺失或解析失败时的回落值：保证 `--build-info` 与界面仍可用，而不是 panic
    fn default() -> Self {
        let unknown = || "unknown".to_string();
        Self {
            branch: unknown(),
            commit_count: unknown(),
            short_hash: unknown(),
            commit_time: unknown(),
            build_time: unknown(),
        }
    }
}

/// 读取构建信息
///
/// - **debug**：按绝对路径运行时读取。文件由 `beforeDevCommand` 每次刷新，因此既不进入
///   编译输入（不触发重编译），又能拿到最新值；文件缺失时回落 `unknown`。
/// - **release**：编译期 `include_str!` 嵌入，产物自包含、不依赖运行时文件；文件变化会
///   触发本 crate 重编译，但 release 只在 CI 构建，无额外代价。
///
/// 两条路径共用同一个路径表达式（`CARGO_MANIFEST_DIR` + `build-info.json`），避免写歪。
#[cfg(debug_assertions)]
fn load_build_info() -> BuildInfo {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/build-info.json"))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

#[cfg(not(debug_assertions))]
fn load_build_info() -> BuildInfo {
    serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/build-info.json")))
        .unwrap_or_default()
}

static APP_INFO: LazyLock<AppInfo> = LazyLock::new(|| {
    let build = load_build_info();
    AppInfo {
        os: constant::sys::OS,
        arch: constant::sys::ARCH,
        branch: build.branch,
        commit_count: build.commit_count,
        short_hash: build.short_hash,
        commit_time: build.commit_time,
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_time: build.build_time,
    }
});

/// 获取 AppInfo 的引用
pub fn app_info() -> &'static AppInfo {
    &APP_INFO
}
