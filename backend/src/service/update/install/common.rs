use anyhow::anyhow;
use semver::Version;
use std::fs::File;
use std::path::{Path, PathBuf};

/// 在 cache/update（优先 bin 子目录，其次根目录）下查找最新 updater-*.exe
///
/// 文件名形如 `updater-0.1.2-windows-x86_64.exe`，版本取文件名中首个可解析的 semver 段；
/// 多个存在时取版本最大者。**代码自发现，不硬编码路径 / 版本**。
pub(in crate::service::update) fn find_updater(cache_dir: &Path) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    dirs.push(cache_dir.join("update").join("bin"));
    dirs.push(cache_dir.join("update"));
    let mut best: Option<(Version, PathBuf)> = None;
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Some(stem) = name.strip_prefix("updater").and_then(|s| s.strip_suffix(".exe")) else {
                continue;
            };
            let Some(ver) = stem.split('-').find_map(|seg| Version::parse(seg).ok()) else {
                continue;
            };
            if best.as_ref().is_none_or(|(bv, _)| ver > *bv) {
                best = Some((ver, path));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// 解压 zip 到 `dest`；若 zip 内是单一顶层目录则返回该目录（解包一层），否则返回 `dest`
///
/// 解压的路径穿越防护被移除，后续有这方面的话需要小心
pub(in crate::service::update) fn extract_zip(zip_path: &Path, dest: &Path) -> anyhow::Result<PathBuf> {
    if dest.exists() {
        std::fs::remove_dir_all(dest).map_err(|e| anyhow!("清理解压目录失败（{}）：{e}", dest.display()))?;
    }
    std::fs::create_dir_all(dest).map_err(|e| anyhow!("创建解压目录失败（{}）：{e}", dest.display()))?;

    let file = File::open(zip_path).map_err(|e| anyhow!("打开下载产物失败（{}）：{e}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| anyhow!("读取 zip 失败（{}）：{e}", zip_path.display()))?;
    archive.extract(dest).map_err(|e| anyhow!("解压更新包失败：{e}"))?;

    // 单顶层目录检测：仅一个条目且为目录 → source 指向它（剥掉外壳层）
    let top: Vec<PathBuf> = std::fs::read_dir(dest)
        .map(|it| it.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    if top.len() == 1 && top[0].is_dir() {
        Ok(top[0].clone())
    } else {
        Ok(dest.to_path_buf())
    }
}

/// 分离式 spawn updater.exe（`CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS`）
///
/// 分离式：updater 不随本进程退出而终止，独立完成更新流程，失败返回 `Err`。
#[cfg(target_os = "windows")]
pub(in crate::service::update) fn spawn_updater(exec_path: &Path, config_path: &Path) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const DETACHED_PROCESS: u32 = 0x0000_0008;

    std::process::Command::new(exec_path)
        .arg(config_path)
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS)
        .spawn()
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("spawn updater.exe 失败: {e}"))
}
