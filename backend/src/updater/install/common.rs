//! 安装器无关的公共能力：解压、Go 更新器 config 骨架。

use crate::util::path_utils;
use anyhow::anyhow;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// 解压 zip 到 `dest`；若 zip 内是单一顶层目录则返回该目录（解包一层），否则返回 `dest`
///
/// 路径与符号链接由 `zip` 的 `extract` 收敛在 `dest` 内（`enclosed_name` + 只允许指向目标目录内的链接）。
pub(crate) fn extract_zip(zip_path: &Path, dest: &Path) -> anyhow::Result<PathBuf> {
    if dest.exists() {
        fs::remove_dir_all(dest).map_err(|e| anyhow!("清理解压目录失败（{}）：{e}", dest.display()))?;
    }
    fs::create_dir_all(dest).map_err(|e| anyhow!("创建解压目录失败（{}）：{e}", dest.display()))?;

    let file = fs::File::open(zip_path).map_err(|e| anyhow!("打开下载产物失败（{}）：{e}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| anyhow!("读取 zip 失败（{}）：{e}", zip_path.display()))?;
    archive.extract(dest).map_err(|e| anyhow!("解压更新包失败：{e}"))?;

    // 单顶层目录检测：仅一个条目且为目录 → source 指向它（剥掉外壳层）
    let top: Vec<PathBuf> = fs::read_dir(dest)
        .map(|it| it.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    if top.len() == 1 && top[0].is_dir() {
        Ok(top[0].clone())
    } else {
        Ok(dest.to_path_buf())
    }
}

/// Go 更新器 config 的公共骨架：`version` / `runtime.log` / `wait` / `launch`
///
/// 两种安装形态只有 `update` 与 `rollback` 不同，由各自的 `compose_config` 补上。
/// `wait` 在 version 3 只认 `pids`（写 `pid` 会被 loader 拒绝）。
pub(crate) fn base_config(pid: u32, log_file: &Path, launch_path: &Path, workspace: &Path) -> Value {
    json!({
        "version": 3,
        "runtime": {
            "headless": false,
            "log": {
                "file": path_utils::to_slash(log_file),
                "level": {
                    "console": "info",
                    "file": "info",
                },
            },
        },
        "wait": {
            "pids": [pid],
            "timeout": 10000,
            "forceKill": true,
            "interval": 500,
        },
        "launch": {
            "execution": {
                "mode": "direct",
                "path": path_utils::to_slash(launch_path),
            },
            "context": {
                "workspace": path_utils::to_slash(workspace),
                "args": [],
                "env": {},
            },
            "lifecycle": {
                "stayAlive": 0,
                "captureOutput": false,
                "captureFormat": "log",
                "captureToFile": true,
            },
        },
    })
}

/// 写 config：补父目录 + pretty 输出（随后由更新器进程一次性读入）
pub(crate) fn write_json(path: &Path, value: &Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| anyhow!("创建更新配置目录失败（{}）：{e}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value)?;
    fs::write(path, text).map_err(|e| anyhow!("写入更新配置失败（{}）：{e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 骨架只含四段，且 `wait.pids` 是数组（version 3 不再认单个 `pid`）
    #[test]
    fn base_config_carries_shared_sections() {
        let config = base_config(
            4321,
            Path::new("C:/app/logs/u/portable-update-1-to-2.log"),
            Path::new("C:/app/rollcaller.exe"),
            Path::new("C:/app"),
        );

        assert_eq!(config["version"], 3);
        assert_eq!(config["wait"]["pids"][0], 4321);
        assert_eq!(config["runtime"]["log"]["file"], "C:/app/logs/u/portable-update-1-to-2.log");
        assert_eq!(config["launch"]["execution"]["path"], "C:/app/rollcaller.exe");
        assert_eq!(config["launch"]["context"]["workspace"], "C:/app");
        assert_eq!(config["launch"]["context"]["args"], json!([]));
        assert!(config["update"].is_null(), "update 由各形态自行补齐");
    }
}
