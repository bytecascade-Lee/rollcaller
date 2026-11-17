//! Develop 直更：把 zip 内的 debug exe 覆盖到编译目录（单文件、不清空、无备份/回滚）。
//!
//! target 是 `backend/target/debug`，其余中间产物必须保留；启动目标用独立文件名
//! （[`DEVELOP_UPDATE_BIN_NAME`]），因为运行中的 `rollcaller.exe` 可删可改名但不可写。

use crate::config::app_paths::AppMode;
use crate::updater::constant::DEVELOP_UPDATE_BIN_NAME;
use crate::updater::install::common::{base_config, extract_zip};
use crate::updater::paths;
use crate::util::path_utils;
use anyhow::anyhow;
use semver::Version;
use serde_json::json;
use std::path::Path;

/// develop zip 内可执行文件的原始名（bundler 的输出名）
const PAYLOAD_BIN_NAME: &str = "rollcaller.exe";

/// Develop 更新：解压 → 载荷归一 → 组装 config → spawn updater（返回即表示调用方应退出）
pub fn install_develop(zip_path: &Path, from: &Version, to: &Version) -> anyhow::Result<()> {
    let exe_path = path_utils::current_exe_clean()?;
    let target_dir = exe_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("无法获取当前可执行文件目录"))?;
    let launch_path = target_dir.join(DEVELOP_UPDATE_BIN_NAME);
    let source_dir = extract_zip(zip_path, &paths::zip_staging(&AppMode::Develop, to))?;
    normalize_payload(&source_dir, &launch_path, &exe_path)?;

    super::start_updater(AppMode::Develop, from, to, |log_file| {
        compose_config(std::process::id(), &source_dir, &target_dir, &launch_path, log_file)
    })
}

/// 载荷归一：source 内的 exe 改名为 [`DEVELOP_UPDATE_BIN_NAME`]，并复制一份到 `launch_path`
fn normalize_payload(source_dir: &Path, launch_path: &Path, exe_path: &Path) -> anyhow::Result<()> {
    let source_exe = source_dir.join(DEVELOP_UPDATE_BIN_NAME);
    if !source_exe.is_file() {
        let extracted = source_dir.join(PAYLOAD_BIN_NAME);
        if !extracted.is_file() {
            return Err(anyhow!(
                "更新包内未找到可执行文件（预期 {} 或 {}）：{}",
                PAYLOAD_BIN_NAME,
                DEVELOP_UPDATE_BIN_NAME,
                source_dir.display()
            ));
        }
        std::fs::rename(&extracted, &source_exe).map_err(|e| {
            anyhow!(
                "重命名更新包内可执行文件失败（{} → {}）：{e}",
                extracted.display(),
                source_exe.display()
            )
        })?;
    }

    if launch_path == exe_path {
        return Ok(());
    }

    // 先删后拷：落点可能是上一轮直更拉起、仍在运行的实例——运行中的镜像可删可改名，但写打开会失败
    match std::fs::remove_file(launch_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(anyhow!("清理旧的更新产物失败（{}）：{e}", launch_path.display())),
    }
    std::fs::copy(&source_exe, launch_path).map_err(|e| {
        anyhow!(
            "复制更新产物到程序目录失败（{} → {}）：{e}",
            source_exe.display(),
            launch_path.display()
        )
    })?;
    Ok(())
}

/// 组装 develop 直更的 config（target 是编译目录，故不清空、不备份、不回滚）
fn compose_config(pid: u32, source_dir: &Path, target_dir: &Path, launch_path: &Path, log_file: &Path) -> serde_json::Value {
    // workspace = target_dir 的上两级（target/debug → cargo workspace 根）。祖先不足两级属布局
    // 错误，允许 panic 但把实际路径打进文案，便于定位。
    let workspace = target_dir.parent().and_then(Path::parent).unwrap_or_else(|| {
        panic!(
            "develop 直更要求 target_dir 至少有两级父目录以推导 workspace，当前 target_dir = {}",
            target_dir.display()
        )
    });

    let mut config = base_config(pid, log_file, launch_path, workspace);
    config["update"] = json!({
        "source": path_utils::to_slash(source_dir),
        "target": path_utils::to_slash(target_dir),
        "preserve": [],
        "cleanBeforeCopy": false,
        "backup": {
            "enabled": false,
        },
    });
    config["rollback"] = json!({
        "enabled": false,
    });
    config
}
