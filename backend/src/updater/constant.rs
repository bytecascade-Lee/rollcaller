//! 更新领域的标量常量

/// 便携版更新器的锁定版本
pub const UPDATER_VERSION: &str = "0.3.0";

// [FIXME]：此处即将被废弃掉，scripts 打包时已不再进行重命名，需要 Rust 侧在解压缩后手动重命名
/// develop 直更产物的文件名
pub const DEVELOP_UPDATE_BIN_NAME: &str = "rollcaller-update-from-develop.exe";

/// 公钥
pub const ROLLCALLER_UPDATE_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IERBNTVDNkMyRUQyMzUxQUQKUldTdFVTUHR3c1pWMnJ3ME55Mkl4d1BpRXpmdmtLOGo0Q3cxeGtrbVJPQjEyQWVyUG5rbGZLS3AK";
