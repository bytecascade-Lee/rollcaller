//! 更新领域的标量常量
//!
//! URL 形状一律在 [`crate::updater::urls`]，路径一律在 [`crate::updater::paths`]——本模块只放
//! "数字与名字"（默认策略、锁定版本、产物名、公钥），不放任何可拼装的模板。

use crate::updater::model::{UpdateChannel, UpdateLevel};

/// 默认更新等级策略：`level = Patch`
pub const DEFAULT_UPDATE_LEVEL: UpdateLevel = UpdateLevel::Patch;
/// 默认更新渠道策略：`channel = Stable`
pub const DEFAULT_UPDATE_CHANNEL: UpdateChannel = UpdateChannel::Stable;

/// 便携版更新器的锁定版本
///
/// 每个 rollcaller 版本固定配对一个更新器版本：我们写出的 config（`version: 3`）由更新器侧解释，
/// 旧更新器会直接拒绝（`config error: unsupported config version 3`，且失败发生在它自己的
/// logger 初始化之前，日志一字不写）。故更新器清单按 **tag** 取，不用 latest
/// （见 [`crate::updater::urls::updater_manifest_url`]）。
///
/// 升级流程：只改本常量，并确认新版本支持的 config 版本 ≥
/// `updater/install/windows/{portable,develop}.rs::compose_config` 写出的版本。
pub const UPDATER_VERSION: &str = "0.3.0";

/// 主包产物的验签公钥（base64(minisign 公钥文本)）
///
/// 由 `tauri signer generate` 产出，`.pub` 文件全文即该值（tauri-cli ≥2.11 已是该格式，勿二次编码）。
/// 生产构建由 [`crate::updater::verify`] 的 `active_pubkey()` 返回它；测试构建改用进程内临时密钥对。
pub const ROLLCALLER_UPDATE_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IERBNTVDNkMyRUQyMzUxQUQKUldTdFVTUHR3c1pWMnJ3ME55Mkl4d1BpRXpmdmtLOGo0Q3cxeGtrbVJPQjEyQWVyUG5rbGZLS3AK";
