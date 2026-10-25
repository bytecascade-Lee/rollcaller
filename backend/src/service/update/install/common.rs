use crate::common::constant::sys::{ARCH, OS};
use crate::common::constant::update::{PLACEHOLDER, UPDATER_MANIFEST_CNB, UPDATER_VERSION};
use crate::common::entity::update::Artifact;
use crate::service::update::paths;
use crate::service::update::verify::{verify_artifact, verify_sha256};
use crate::state::http_client;
use anyhow::{anyhow, Context};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// 锁定版本的更新器清单地址
fn updater_manifest_url() -> String {
    UPDATER_MANIFEST_CNB.replace(PLACEHOLDER, UPDATER_VERSION)
}

/// 清单里属于当前平台的那一项（产物名 / sha256 / 字节数 / 签名都由清单给）
fn platform_artifact(manifest: &serde_json::Value) -> anyhow::Result<Artifact> {
    let os = OS.to_string().to_ascii_lowercase();
    let arch = ARCH.to_string().to_ascii_lowercase();
    let entry = &manifest[os.as_str()][arch.as_str()];
    let url = entry["url"]
        .as_str()
        .ok_or_else(|| anyhow!("更新器清单缺少 {os}.{arch}.url：{manifest}"))?;
    Ok(Artifact {
        url: url.parse().context(anyhow!("更新器下载地址不是合法 URL：{url}"))?,
        sha256: entry["sha256"]
            .as_str()
            .ok_or_else(|| anyhow!("更新器清单缺少 {os}.{arch}.sha256"))?
            .to_string(),
        // 更新器目前只发 sha256；清单哪天给出 signature，校验就自动改走“签名必验”
        signature: entry["signature"].as_str().unwrap_or_default().to_string(),
        size: entry["size"]
            .as_u64()
            .ok_or_else(|| anyhow!("更新器清单缺少 {os}.{arch}.size"))?,
    })
}

/// 校验产物：有签名一并验，没有则只验 sha256
fn verify_payload(bytes: &[u8], artifact: &Artifact) -> anyhow::Result<()> {
    if artifact.signature.is_empty() {
        verify_sha256(bytes, &artifact.sha256)
    } else {
        verify_artifact(bytes, artifact)
    }
}

/// 文件名是否属于锁定版本（`updater-0.3.0-windows-x86_64.exe`）
fn is_pinned_name(name: &str) -> bool {
    name.contains(&format!("-{UPDATER_VERSION}-"))
}

/// 本地锁定版本的更新器（只看文件名里的版本段：不联网、不执行）
pub(in crate::service::update) fn updater_exe() -> Option<PathBuf> {
    fs::read_dir(paths::updater_bin_dir())
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.extension().and_then(|ext| ext.to_str()) == Some("exe")
                && path.file_name().and_then(|name| name.to_str()).is_some_and(is_pinned_name)
        })
}

/// 删掉缓存里除锁定产物以外的所有 exe
fn discard_others(keep: &Path) {
    let Some(entries) = keep.parent().and_then(|dir| fs::read_dir(dir).ok()) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path != keep && path.extension().and_then(|ext| ext.to_str()) == Some("exe") {
            let _ = fs::remove_file(&path);
        }
    }
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

/// 分离式 spawn updater.exe
#[cfg(target_os = "windows")]
pub(in crate::service::update) fn spawn_updater(
    exec_path: &Path,
    config_path: &Path,
    log_file: &Path,
) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const DETACHED_PROCESS: u32 = 0x0000_0008;

    let output_path = log_file.with_extension("stderr.log");
    let stdout =
        fs::File::create(&output_path).map_err(|e| anyhow!("创建更新器输出日志失败（{}）：{e}", output_path.display()))?;
    let stderr = stdout
        .try_clone()
        .map_err(|e| anyhow!("复制更新器输出日志句柄失败（{}）：{e}", output_path.display()))?;

    Command::new(exec_path)
        .arg(config_path)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS)
        .spawn()
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("spawn updater.exe 失败: {e}"))
}

/// 便携版更新器就绪（安装前 ensure；幂等）
///
/// 本地已有锁定版本 → 直接用，不联网；没有才拉该版本 tag 的清单，按清单下载 / 校验 / 落盘。
/// 两条路径最后都把目录收敛到只剩锁定版本。触发点 = Portable / Develop 安装前的编排层异步段。
#[cfg(target_os = "windows")]
pub async fn ensure_updater() -> anyhow::Result<PathBuf> {
    // 1. 本地已有 → 直接复用（不拉清单）
    if let Some(local) = updater_exe() {
        discard_others(&local);
        return Ok(local);
    }

    // 2. 拉锁定版本（tag）的清单
    let text = http_client::client()
        .get(updater_manifest_url())
        .header("Accept", "application/json")
        .send()
        .await
        .context(anyhow!("拉取更新器清单失败：网络错误"))?
        .error_for_status()
        .context(anyhow!("拉取更新器清单失败：服务器返回错误状态"))?
        .text()
        .await
        .context(anyhow!("拉取更新器清单失败：读取响应失败"))?;
    let manifest: serde_json::Value = serde_json::from_str(&text).context(anyhow!("更新器清单不是合法 JSON"))?;

    // 清单自报版本必须就是锁定的那个（tag 被重发时能立刻发现）
    let version = manifest["version"].as_str().unwrap_or_default();
    if version != UPDATER_VERSION {
        anyhow::bail!("更新器清单版本不符：期望 {UPDATER_VERSION}，实际 {version:?}");
    }
    let artifact = platform_artifact(&manifest)?;
    let file_name = artifact
        .file_name()
        .ok_or_else(|| anyhow!("更新器下载地址缺少文件名：{}", artifact.url))?;
    let dest = paths::portable_updater_bin(&file_name);

    // 3. 下载（体积小，内存收齐后整体校验再落盘）
    let bytes = http_client::download()
        .get(artifact.url.clone())
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

    verify_payload(&bytes, &artifact).context(anyhow!("更新器校验失败（锁定版本 {UPDATER_VERSION}）"))?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| anyhow!("创建更新器目录失败（{}）：{e}", parent.display()))?;
    }
    fs::write(&dest, bytes).map_err(|e| anyhow!("写入更新器失败（{}）：{e}", dest.display()))?;

    // 4. 只留锁定版本
    discard_others(&dest);
    Ok(dest)
}

/// 非 Windows 平台：便携版安装不可用，更新器亦无需下载
#[cfg(not(target_os = "windows"))]
pub async fn ensure_updater() -> anyhow::Result<PathBuf> {
    anyhow::bail!("便携版安装仅支持 Windows")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 清单地址由锁定版本拼出，不留占位符
    #[test]
    fn manifest_url_uses_pinned_version() {
        let url = updater_manifest_url();
        assert!(!url.contains(PLACEHOLDER), "占位符未替换：{url}");
        assert!(url.contains(&format!("/v{UPDATER_VERSION}/")), "tag 不符：{url}");
    }

    /// 清单解析：取当前平台那一项，签名缺省为空（将来有则照抄）
    #[test]
    fn platform_artifact_follows_manifest() {
        let os = OS.to_string().to_ascii_lowercase();
        let arch = ARCH.to_string().to_ascii_lowercase();
        let mut manifest = serde_json::json!({ "version": UPDATER_VERSION });
        manifest[os.as_str()] = serde_json::json!({
            arch.as_str(): {
                "url": format!("https://example.com/updater-{UPDATER_VERSION}-{os}-{arch}.exe"),
                "sha256": "a".repeat(64),
                "size": 1234,
            }
        });

        let artifact = platform_artifact(&manifest).expect("清单应有当前平台项");
        assert_eq!(
            artifact.file_name().unwrap(),
            format!("updater-{UPDATER_VERSION}-{os}-{arch}.exe")
        );
        assert_eq!(artifact.size, 1234);
        assert!(artifact.signature.is_empty(), "清单没给签名时留空");

        // 缺当前平台项 → 报错而不是拿空值去下载
        assert!(platform_artifact(&serde_json::json!({ "version": UPDATER_VERSION })).is_err());
    }

    /// 本地判定：文件名里带锁定版本才算（改名/换平台都认），其他版本不算
    #[test]
    fn pinned_name_matches_only_locked_version() {
        assert!(is_pinned_name("updater-0.3.0-windows-x86_64.exe"));
        assert!(is_pinned_name(&format!("updater-{UPDATER_VERSION}-macos-arm64")));
        assert!(!is_pinned_name("updater-0.2.0-windows-x86_64.exe"));
        assert!(!is_pinned_name("updater-windows-x86_64.exe"));
    }

    /// 只留锁定产物：其他 exe 删掉，非 exe 不动
    #[test]
    fn discard_others_keeps_only_pinned() {
        let dir = std::env::temp_dir().join(format!("rollcaller-updaters-{}", &uuid::Uuid::new_v4().to_string()[..8]));
        std::fs::create_dir_all(&dir).expect("创建临时目录");
        let keep = dir.join("updater-0.3.0-windows-x86_64.exe");
        std::fs::write(&keep, b"keep").unwrap();
        std::fs::write(dir.join("updater-0.2.0-windows-x86_64.exe"), b"old").unwrap();
        std::fs::write(dir.join("leftover.part"), b"part").unwrap();

        discard_others(&keep);

        assert!(keep.is_file(), "锁定产物必须保留");
        assert!(!dir.join("updater-0.2.0-windows-x86_64.exe").exists(), "旧版本必须删除");
        assert!(dir.join("leftover.part").exists(), "非 exe 不归本函数管");
        std::fs::remove_dir_all(&dir).ok();
    }
}
