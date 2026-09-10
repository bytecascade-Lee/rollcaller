//! 自动更新模块的路径管理
//!
//! 本模块集中定义自动更新功能所需的所有文件和目录路径，遵循"路径即服务"的设计理念：
//! 调用方只传可变信息（文件名 / 版本 / 更新源），目录根一律由本模块自 [`app_paths`] 取得，
//! 避免字符串占位符，确保编译期类型安全。
//!
//! # 布局原则
//!
//! - **产物平铺、文件名自带完整信息**：主包产物（nsis 安装包 / portable zip / 开发模式下 debug 产物）
//!   与更新器都不再按版本建子目录，文件名携带版本 / 平台 / 架构等
//!   完整信息（通常取发布产物 url 的最后一段，如 `rollcaller-0.1.2-windows-x86_64-setup.exe`），
//!   文件名即一次下载目标的唯一标识。
//! - 目录按内容的**生命周期与语义**分派：
//!   - `cache/update/{source}/{version}.json`：目标版本清单缓存（持久复用，按源 + 版本寻址）；
//!   - `cache/update/bin/`：更新器 exe（工具属性，缓存复用，按文件名区分版本）；
//!   - `temp/update/packages/`：已下载待安装的主包产物（临时，安装即弃，同目录可并存他版本残留）；
//!   - `temp/update/config/{mode}-config-{from}-to-{to}.json`：更新器安装会话配置（见 [`updater_config`]）；
//!   - `temp/downloads/`：下载中的 `.part` 工作区（不完整、随时因不合法而删除；未来
//!     断点续传 / 多进程下载的临时文件也在此，与正式产物隔离，不同目录下 rename 同卷原子）；
//!   - `temp/update/staging/{mode}-source-{version}/`：压缩包解压暂存（安装过程专属，更新器完成后清理）。

use crate::common::enums::update::UpdateSource;
use crate::config::app_paths::{cache_dir, temp_dir, AppMode};
use semver::Version;
use std::path::PathBuf;

/// 指定更新源的版本清单缓存路径
///
/// 目标版本的发布清单（`latest-{source}.json`）拉取后落盘于此缓存；
/// 下次检查再次选中同一目标版本时直接读缓存、不再走网络（版本索引 versions.json
/// 每次仍照常拉取以感知新版本）。
///
/// # 参数
/// - `source`：更新源（GitHub / CNB / Develop），转小写作为目录名；
/// - `version`：目标语义化版本号，用于唯一标识一个发布版本。
///
/// # 返回
/// `cache_dir/update/{source}/{version}.json`。
///
/// # 例
/// `manifest(&UpdateSource::Github, v1.2.3)` → `.../cache/update/github/1.2.3.json`
///
pub fn manifest(source: &UpdateSource, version: &Version) -> PathBuf {
    cache_dir().join(format!(
        "update/{}/{version}.json",
        source.to_string().to_ascii_lowercase(),
    ))
}

/// 生成下载中的临时碎片文件（`.part`）路径
///
/// 下载过程中的数据暂存于 `temp/downloads/`，与正式产物（[`package`] 的 packages 目录）
/// **刻意分离**：part 是不完整的工作区产物，可能随时因校验失败 / 取消 / 中断而被删除，
/// 未来断点续传与多进程下载的临时文件也共居此处，不应与"已校验的正式产物"混放。
/// 下载完成并校验通过后，才由调用方 rename 为正式文件名迁入 packages
/// （两者同在 app 自管 temp 卷内，跨目录 rename 原子）。
///
/// 每次调用生成不同的文件名（随机 simple uuid v4），多下载任务之间天然隔离。
///
/// # 返回
/// `temp_dir/downloads/{随机uuid}.part`
///
pub fn part() -> PathBuf {
    temp_dir().join(format!("downloads/{}.part", uuid::Uuid::new_v4().to_string().replace("-", "")))
}

/// 下载完成的安装包 / 压缩包的存放路径（主包产物）
///
/// 主包产物（nsis 安装包 `.exe` / portable zip `.zip`）**平铺**存放于
/// `temp/update/packages/`，不按版本分目录。文件名的语义由调用方保证——应为发布
/// 产物 url 的最后一段，自带版本 / 平台 / 架构等完整信息
/// （如 `rollcaller-0.1.2-windows-x86_64-setup.exe`），文件名即一次下载目标的唯一标识：
/// 就绪探测、复验与取消清理均按此文件名寻址，无需额外的版本目录。
/// 同目录允许存在他版本产物残留，由 app 自管 temp 的清理规则兜底。
///
/// # 参数
/// - `file_name`：文件全名（含扩展名，带版本等完整信息）。
///
/// # 返回
/// `temp_dir/update/packages/{file_name}`
///
/// # 注意
/// 本路径仅表示存放位置，不负责实际下载或重命名操作。
///
pub fn package(file_name: &str) -> PathBuf {
    temp_dir().join(format!("update/packages/{}", file_name))
}

/// 更新器（Go updater）可执行文件的存放路径
///
/// 更新器与主包产物同风格命名：文件名自带完整信息
/// （如 `updater-0.1.2-windows-x86_64.exe`），平铺存放于 `cache/update/bin/` 下、
/// 按文件名区分版本。调用方传入的文件名通常取更新器产物 url 的最后一段。
/// 便携版更新器被缓存于此，负责执行实际的文件替换操作。
///
/// # 参数
/// - `file_name`：更新器完整文件名（含 `.exe` 扩展名，带版本等完整信息）。
///
/// # 返回
/// `cache_dir/update/bin/{file_name}`。
///
/// # 例
/// `portable_updater_bin("updater-1.2.3-windows-x86_64.exe")` → `.../cache/update/bin/updater-1.2.3-windows-x86_64.exe`
///
pub fn portable_updater_bin(file_name: &str) -> PathBuf {
    cache_dir().join(format!("update/bin/{file_name}"))
}

/// 便携版解压的暂存根目录
///
/// 对于便携版（Portable）更新，下载的 zip 产物被解压到该目录下，解压后的内容
/// 与安装后的应用程序文件结构完全一致；Go updater 将从此目录复制文件到应用根
/// 目录实现覆盖更新（updater 不解压 zip，source 必须是已解压目录）。
///
/// # 参数
/// - `version`：目标语义化版本号，用于区分不同版本的解压内容，
///   防止旧版本残留影响新版本升级。
///
/// # 返回
/// `temp_dir/update/staging/portable-source-{version}/`
///
/// # 注意
/// - 该目录仅在更新过程中存在，更新完成后应由更新器负责清理；
/// - 解压时建议采用"剥离顶层目录"策略，使内容直接位于该目录下。
///
pub fn zip_staging(mode: &AppMode, version: &Version) -> PathBuf {
    temp_dir().join(format!("update/staging/{mode}-source-{version}"))
}

/// Go updater（便携版更新器）的安装会话配置文件路径
///
/// config.json 组装后写盘于此（`temp/update/config` 下），随 spawn 的 updater.exe 传入；
/// 文件名以 `from → to` 标识一次安装，同一对版本重试时同名覆盖（wait.pid 等
/// 运行时字段每次重写）。与解压内容（[`zip_staging`]）分居，避免被
/// updater 当作 source 一并复制进 target。
pub fn updater_config(mode: &AppMode, from: &Version, to: &Version) -> PathBuf {
    temp_dir().join(format!("update/config/{mode}-config-{from}-to-{to}.json"))
}
