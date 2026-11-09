//! 便携版安装：解压 zip → 组装 Go updater config → 分离式 spawn。
//!
//! Go updater 不会自动解压 zip，`update.source` 必须是已解压目录；config 与解压内容必须分居
//! 两个目录，否则会被当作 source 一起复制进 target。

use crate::config::app_paths::AppMode;
use crate::updater::install::common::{base_config, extract_zip};
use crate::updater::paths;
use crate::util::path_utils;
use anyhow::anyhow;
use semver::Version;
use serde_json::json;
use std::path::Path;

/// 便携版完整安装编排（返回即表示更新器已启动，调用方应随即退出）
pub fn install_portable(zip_path: &Path, from: &Version, to: &Version) -> anyhow::Result<()> {
    let exe_path = path_utils::current_exe_clean()?;
    let target_dir = exe_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("无法获取当前可执行文件目录"))?;
    let data_dir = target_dir.join("data");
    let source_dir = extract_zip(zip_path, &paths::zip_staging(&AppMode::Portable, to))?;
    // 备份目录不必预建：Go updater 复制前会自行 MkdirAll
    let backup_dir = paths::backup(&AppMode::Portable, from, to);

    super::start_updater(AppMode::Portable, from, to, |log_file| {
        compose_config(
            std::process::id(),
            &source_dir,
            &target_dir,
            &data_dir,
            &backup_dir,
            &exe_path,
            log_file,
        )
    })
}

/// 组装 Go updater 的完整 config（version 3；字段写全、路径转正斜杠）
///
/// `backup.location` 落在 `data` 树内（Portable 的 `temp_dir` = `exe_dir/data/temp`），同时命中
/// `backup.exclude` 与 `update.preserve`：前者防止备份遍历自己，后者防止备份被清理阶段删掉。
fn compose_config(
    pid: u32,
    source_dir: &Path,
    target_dir: &Path,
    data_dir: &Path,
    backup_dir: &Path,
    exe_path: &Path,
    log_file: &Path,
) -> serde_json::Value {
    let mut config = base_config(pid, log_file, exe_path, target_dir);
    config["update"] = json!({
        "source": path_utils::to_slash(source_dir),
        "target": path_utils::to_slash(target_dir),
        "preserve": [path_utils::to_slash(data_dir)],
        "cleanBeforeCopy": true,
        "backup": {
            "enabled": true,
            "location": path_utils::to_slash(backup_dir),
            "exclude": [path_utils::to_slash(data_dir)],
            "cleanupOnSuccess": false,
        },
    });
    config["rollback"] = json!({
        "enabled": true,
        "fallbackExecutable": path_utils::to_slash(exe_path),
        "maxAttempts": 2,
    });
    config
}
