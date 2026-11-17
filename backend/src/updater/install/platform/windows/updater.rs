//! 便携 / 直更共用的 Go 更新器：本地发现、缺失时按锁定 tag 拉取、只留锁定版本。

use crate::common::constant::sys::{ARCH, OS};
use crate::state::http_client;
use crate::updater::constant::UPDATER_VERSION;
use crate::updater::model::Artifact;
use crate::updater::paths;
use crate::updater::urls;
use crate::updater::verify::{verify_artifact, verify_sha256};
use crate::util::fs_utils;
use anyhow::{anyhow, Context};
use std::fs;
use std::path::{Path, PathBuf};

/// 确保锁定版本的更新器就绪（幂等）
pub async fn ensure() -> anyhow::Result<()> {
    let updater = match find() {
        Some(local) => local,
        None => fetch().await?,
    };
    prune(&updater);
    Ok(())
}

/// 本地锁定版本的更新器（只看文件名与大小：不联网、不执行）
pub fn find() -> Option<PathBuf> {
    find_in(&paths::updater_bin_dir())
}

fn find_in(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| is_updater_name(path) && is_non_empty(path))
}

/// 文件名是否属于锁定版本（`updater-0.3.0-windows-x86_64.exe`）
fn is_pinned_name(name: &str) -> bool {
    name.contains(&format!("-{UPDATER_VERSION}-"))
}

fn is_updater_name(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("exe")
        && path.file_name().and_then(|name| name.to_str()).is_some_and(is_pinned_name)
}

/// 空文件是"上次写入被中断"的痕迹，不能当成可用更新器
fn is_non_empty(path: &Path) -> bool {
    path.metadata().map(|meta| meta.len() > 0).unwrap_or(false)
}

/// 拉锁定 tag 的清单 → 下载 → 大小与签名 → 原子落盘
async fn fetch() -> anyhow::Result<PathBuf> {
    let manifest = fetch_manifest().await?;
    let artifact = platform_artifact(&manifest)?;
    let file_name = artifact
        .file_name()
        .ok_or_else(|| anyhow!("更新器下载地址缺少文件名：{}", artifact.url))?;
    let dest = paths::updater_bin_file(&file_name);

    // 体积小，内存收齐后整体校验再落盘
    let bytes = download(&artifact).await?;
    if bytes.len() as u64 != artifact.size {
        anyhow::bail!("更新器大小与清单不符：期望 {} 字节，实际 {} 字节", artifact.size, bytes.len());
    }
    verify_payload(&bytes, &artifact).context(anyhow!("更新器校验失败（锁定版本 {UPDATER_VERSION}）"))?;
    fs_utils::write_atomic(&dest, &bytes).map_err(|e| anyhow!("写入更新器失败（{}）：{e}", dest.display()))?;
    Ok(dest)
}

/// 锁定 tag 的清单字节；自报版本必须是锁定的那个（tag 被重发时能立刻发现）
async fn fetch_manifest() -> anyhow::Result<serde_json::Value> {
    let text = http_client::client()
        .get(urls::updater_manifest_url(UPDATER_VERSION))
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
    let version = manifest["version"].as_str().unwrap_or_default();
    if version != UPDATER_VERSION {
        anyhow::bail!("更新器清单版本不符：期望 {UPDATER_VERSION}，实际 {version:?}");
    }
    Ok(manifest)
}

async fn download(artifact: &Artifact) -> anyhow::Result<Vec<u8>> {
    let response = http_client::download()
        .get(artifact.url.clone())
        .send()
        .await
        .context(anyhow!("下载更新器失败：网络错误"))?
        .error_for_status()
        .context(anyhow!("下载更新器失败：服务器返回错误状态"))?;
    let bytes = response.bytes().await.context(anyhow!("下载更新器失败：读取响应失败"))?;
    Ok(bytes.to_vec())
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
        // 更新器目前只发 sha256；清单哪天给出 signature，校验就自动改走"签名必验"
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

/// 只留锁定产物：同目录其它 exe 一律删掉
fn prune(keep: &Path) {
    let Some(entries) = keep.parent().and_then(|dir| fs::read_dir(dir).ok()) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path != keep && path.extension().and_then(|ext| ext.to_str()) == Some("exe") {
            let _ = fs::remove_file(&path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 更新器清单按锁定版本的 tag 取，不用 latest
    #[test]
    fn updater_manifest_url_is_pinned_to_locked_version() {
        let url = urls::updater_manifest_url(UPDATER_VERSION);
        assert!(url.contains(&format!("/v{UPDATER_VERSION}/")), "tag 不符：{url}");
        assert!(!url.contains("latest/download"), "不得退化成 latest：{url}");
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

    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rollcaller-updaters-{tag}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("创建临时目录");
        dir
    }

    /// 只留锁定产物：其他 exe 删掉，非 exe 不动
    #[test]
    fn prune_keeps_only_pinned() {
        let dir = scratch_dir("prune");
        let keep = dir.join("updater-0.3.0-windows-x86_64.exe");
        fs::write(&keep, b"keep").unwrap();
        fs::write(dir.join("updater-0.2.0-windows-x86_64.exe"), b"old").unwrap();
        fs::write(dir.join("leftover.part"), b"part").unwrap();

        prune(&keep);

        assert!(keep.is_file(), "锁定产物必须保留");
        assert!(!dir.join("updater-0.2.0-windows-x86_64.exe").exists(), "旧版本必须删除");
        assert!(dir.join("leftover.part").exists(), "非 exe 不归本函数管");
        fs::remove_dir_all(&dir).ok();
    }

    /// 空文件是写入中断的痕迹，不能被当成可用更新器
    #[test]
    fn find_skips_empty_and_foreign_files() {
        let dir = scratch_dir("find");
        fs::write(dir.join("updater-0.2.0-windows-x86_64.exe"), b"old").unwrap();
        fs::write(dir.join("notes.txt"), b"x").unwrap();
        fs::write(dir.join(format!("updater-{UPDATER_VERSION}-windows-x86_64.exe")), b"").unwrap();
        assert!(find_in(&dir).is_none(), "空文件与旧版本都不算");

        let pinned = dir.join(format!("updater-{UPDATER_VERSION}-windows-x86_64.exe"));
        fs::write(&pinned, b"binary").unwrap();
        assert_eq!(find_in(&dir).as_deref(), Some(pinned.as_path()));
        fs::remove_dir_all(&dir).ok();
    }
}
