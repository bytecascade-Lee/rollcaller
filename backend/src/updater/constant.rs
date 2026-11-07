//! 更新领域的标量常量

/// 便携版更新器的锁定版本
pub const UPDATER_VERSION: &str = "0.3.0";

/// develop 直更落进程序目录的文件名
///
/// 打包侧产出的是 bundler 的 `rollcaller.exe`（`develop.rs::PAYLOAD_BIN_NAME`），客户端解压后
/// 改名到此名：运行中的 `rollcaller.exe` 可改名可删除但不可写。**两侧没有一致性要求。**
pub const DEVELOP_UPDATE_BIN_NAME: &str = "rollcaller-update-from-develop.exe";

/// 公钥
pub const ROLLCALLER_UPDATE_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IERBNTVDNkMyRUQyMzUxQUQKUldTdFVTUHR3c1pWMnJ3ME55Mkl4d1BpRXpmdmtLOGo0Q3cxeGtrbVJPQjEyQWVyUG5rbGZLS3AK";
