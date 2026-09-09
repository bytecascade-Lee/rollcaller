use crate::common::constant::sys::{ARCH, OS};
use crate::common::constant::update::PORTABLE_UPDATER_LATEST_MANIFEST_CNB;
use crate::common::entity::update::Artifact;
use crate::service::update::paths;
use crate::service::update::verify::verify_sha256;
use crate::state::http_client;
use anyhow::{anyhow, Context};
use regex::Regex;
use semver::Version;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(in crate::service::update) fn find_updater() -> Option<PathBuf> {
    // 1. 收集所有 .exe 文件
    let mut exe_paths = Vec::new();
    let dir = paths::portable_updater_bin("FILE_NAME").parent().unwrap().to_path_buf();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("exe") {
                exe_paths.push(path);
            }
        }
    }


    if exe_paths.is_empty() {
        return None;
    }

    // 2. 对每个 exe 获取版本，并记录有效项
    let mut versioned = Vec::new(); // (Version, PathBuf)
    let mut to_delete = Vec::new(); // 存放需要删除的路径（包括无效和未被选中的）

    // 预编译正则，用于从输出中提取 semver
    let regex = Regex::new(r"\d+\.\d+\.\d+(?:-[.\w]+)?").unwrap();

    for exe in &exe_paths {
        match Command::new(exe).arg("-v").output() {
            Ok(output) => {
                // 尝试从 stdout 中提取版本
                let stdout = String::from_utf8_lossy(&output.stdout);

                if let Some(ver_str) = regex.find(&stdout).map(|m| m.as_str()) {
                    if let Ok(ver) = Version::parse(ver_str) {
                        versioned.push((ver, exe.clone()));
                        continue; // 成功解析，不加入删除列表
                    }
                }
                // 解析失败，标记删除
                to_delete.push(exe.clone());
            }
            Err(_) => {
                // 执行失败，标记删除
                to_delete.push(exe.clone());
            }
        }
    }

    // 如果没有有效版本，删除所有并返回 None
    if versioned.is_empty() {
        for exe in exe_paths {
            let _ = fs::remove_file(exe);
        }
        return None;
    }

    // 3. 选出最大版本
    let (best_ver, best_path) = versioned.into_iter().max_by(|(v1, _), (v2, _)| v1.cmp(v2)).unwrap();

    // 4. 删除所有其他 exe（包括无效的和未被选中的有效版本）
    for exe in exe_paths {
        if exe != best_path {
            let _ = fs::remove_file(&exe);
        }
    }

    // 5. 重命名保留文件，使其版本段与实际版本一致
    if let Some(file_name) = best_path.file_name().and_then(|n| n.to_str()) {
        // 期望格式：<name>-<version>-<platform>-<arch>.exe
        // 我们提取 name, platform, arch，并替换版本段
        let parts: Vec<&str> = file_name.split('-').collect();
        return if parts.len() >= 4 {
            let name = parts[0];
            let platform = parts[parts.len() - 2]; // 倒数第二个
            let arch = parts[parts.len() - 1].trim_end_matches(".exe");
            let new_name = format!("{}-{}-{}-{}.exe", name, best_ver, platform, arch);
            let new_path = best_path.with_file_name(&new_name);
            if new_path != best_path {
                let _ = fs::rename(&best_path, &new_path);
                Some(new_path)
            } else {
                Some(best_path)
            }
        } else {
            // 文件名格式不符合预期，保留原名
            Some(best_path)
        };
    }

    Some(best_path)
}
/// 解压 zip 到 `dest`；若 zip 内是单一顶层目录则返回该目录（解包一层），否则返回 `dest`
///
/// 解压的路径穿越防护被移除，后续有这方面的话需要小心
pub(in crate::service::update) fn extract_zip(zip_path: &Path, dest: &Path) -> anyhow::Result<PathBuf> {
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

/// 分离式 spawn updater.exe（`CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS`）
///
/// 分离式：updater 不随本进程退出而终止，独立完成更新流程，失败返回 `Err`。
#[cfg(target_os = "windows")]
pub(in crate::service::update) fn spawn_updater(exec_path: &Path, config_path: &Path) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const DETACHED_PROCESS: u32 = 0x0000_0008;

    Command::new(exec_path)
        .arg(config_path)
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS)
        .spawn()
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("spawn updater.exe 失败: {e}"))
}

/// 便携版更新器就绪（安装前 ensure；幂等）
///
/// 简单拉取 + 下载：**无进度、无取消、不涉状态机**。触发点 = Portable 安装前
/// （编排层 `install` 异步段），由 [`super::ensure_updater`] 按形态转发到此。
///
/// # 流程
///
/// 1. [`find_updater`] 在 cache 已发现更新器 → 直接返回（幂等，不重复下载）；
/// 2. 拉更新器仓库 latest manifest（与主源一致用 CNB，[`PORTABLE_UPDATER_LATEST_MANIFEST_CNB`]）；
/// 3. 更新器清单是**扁平结构**（`{version, publishDate, windows:{x86_64|arm64:{url,sha256,size}}}`，
///    无 signature / severity / platforms 包装），故不建新结构体，直接 `serde_json::Value`
///    按 `data["windows"][arch]` 索引。url 文件名须为 `updater-{semver}-windows-{arch}.exe`
///    （[`find_updater`] 按该命名解析版本号）。
///    注：本函数刻意不转 `Artifact` 走标准 verify 管线——落点是持久 cache（非 temp
///    packages）、且仅 sha256；**一旦更新器支持 minisign，本函数即废弃**，届时走
///    标准 Artifact + verify 管线（`verify_artifact_path` 有空签名语义，只填字段、零改动）。
/// 4. 下载到 [`paths::portable_updater_bin`]（url 最后一段），内存收齐后整体校验再落盘
///    ——磁盘上从不出现"未校验的正式产物"。
#[cfg(target_os = "windows")]
pub async fn ensure_updater() -> anyhow::Result<PathBuf> {
    // 1. 已存在任意版本 → 幂等返回
    if let Some(existing) = find_updater() {
        return Ok(existing);
    }

    // 2. 拉更新器最新清单（与主源一致的 CNB 源）
    let text = http_client::client()
        .get(PORTABLE_UPDATER_LATEST_MANIFEST_CNB)
        .header("Accept", "application/json")
        .send()
        .await
        .context(anyhow!("拉取更新器清单失败：网络错误"))?
        .error_for_status()
        .context(anyhow!("拉取更新器清单失败：服务器返回错误状态"))?
        .text()
        .await
        .context(anyhow!("拉取更新器清单失败：读取响应失败"))?;

    // 3. 按当前架构索引扁平清单
    let value: serde_json::Value = serde_json::from_str(&text).context(anyhow!("更新器清单不是合法 JSON"))?;
    let entry = &value["data"][OS.to_string().to_ascii_lowercase()][ARCH.to_string().to_ascii_lowercase()];
    let url_str = entry["url"].as_str().context(anyhow!(format!(
        "更新器清单缺少 windows.{}.url",
        ARCH.to_string().to_ascii_lowercase()
    )))?;

    let artifact = Artifact {
        url: url_str.parse().context(anyhow!("更新器下载地址不是合法 URL：{url_str}"))?,
        sha256: entry["sha256"]
            .as_str()
            .context(anyhow!(format!(
                "更新器清单缺少 windows.{}.sha256",
                ARCH.to_string().to_ascii_lowercase()
            )))?
            .to_string(),
        signature: "".to_string(),
        size: entry["size"].as_u64().context("更新器清单没有字节数")?,
    };

    let file_name = artifact
        .file_name()
        .ok_or_else(|| anyhow!("更新器下载地址缺少文件名：{url_str}"))?;
    let dest = paths::portable_updater_bin(&file_name);

    // 4. 下载（体积小，内存收齐后整体校验再落盘）
    let bytes = http_client::download()
        .get(artifact.url)
        .send()
        .await
        .context(anyhow!("下载更新器失败：网络错误"))?
        .error_for_status()
        .context(anyhow!("下载更新器失败：服务器返回错误状态"))?
        .bytes()
        .await
        .context(anyhow!("下载更新器失败：读取响应失败"))?;

    if bytes.len() as u64 != artifact.size {
        anyhow::bail!("更新器大小与清单不符：期望 {} 字节，实际 {} 字节", artifact.size, bytes.len());
    }

    verify_sha256(&bytes, &artifact.sha256).context(anyhow!("更新器校验失败"))?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| anyhow!("创建更新器目录失败（{}）：{e}", parent.display()))?;
    }
    fs::write(&dest, bytes).map_err(|e| anyhow!("写入更新器失败（{}）：{e}", dest.display()))?;
    Ok(dest)
}

/// 非 Windows 平台：便携版安装不可用，更新器亦无需下载
#[cfg(not(target_os = "windows"))]
pub async fn ensure_updater() -> anyhow::Result<PathBuf> {
    anyhow::bail!("便携版安装仅支持 Windows")
}
