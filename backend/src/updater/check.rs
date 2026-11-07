//! 更新检查：拉版本索引挑"最高可安装版本" -> 拉该版本清单（缓存优先）-> 一致性复判 -> 组装命中结果

use crate::common::constant::sys::{ARCH, OS};
use crate::config::app_paths::AppMode;
use crate::updater::model::{FoundUpdate, Policy, UpdateDecision, UpdateInfo, UpdateManifest, UpdateSource};
use crate::updater::remote::{fetch, index, manifest};
use crate::updater::urls::{manifest_url, versions_index_url};
use crate::updater::{decide, paths};
use anyhow::anyhow;
use reqwest::Client;
use semver::Version;

/// 检查是否有可用更新（`Ok(None)` = 无更新）
///
/// `versions.json` 每次发布都新增条目，故每次都拉、不缓存
/// 目标版本清单按"源 + 版本"寻址，命中缓存且摘要匹配时免网络（见 [`manifest::load`]）
pub async fn check(
    client: &Client,
    source: UpdateSource,
    current: &Version,
    policy: &Policy,
    mode: AppMode,
) -> anyhow::Result<Option<FoundUpdate>> {
    let index_text = fetch::fetch_json_text(client, &versions_index_url(source)).await?;
    let candidates = index::parse_index(&index_text)?;

    let Some(target) = index::pick_target(current, &candidates, policy)? else {
        return Ok(None);
    };

    let url = manifest_url(source, &target.version);
    let cache_path = paths::manifest(&source, &target.version);
    let bytes = manifest::load(client, &url, &cache_path, target.manifests.get(source)).await?;
    let manifest = manifest::parse_manifest(&bytes)?;

    validate(current, policy, &target.version, &manifest, mode)
}

/// 一致性复判 + 取当前形态产物 + 组装命中结果
fn validate(
    current: &Version,
    policy: &Policy,
    expected: &Version,
    manifest: &UpdateManifest,
    mode: AppMode,
) -> anyhow::Result<Option<FoundUpdate>> {
    if manifest.version != *expected {
        return Err(anyhow!(
            "版本索引与清单不一致：索引指向 {expected}，清单返回 {}",
            manifest.version
        ));
    }
    match decide::evaluate(current, &manifest.version, policy, manifest.severity) {
        UpdateDecision::Skip => {
            return Err(anyhow!(
                "版本索引与清单不一致：{} 的清单标注的 severity 不足以放行本次更新",
                manifest.version
            ))
        }
        UpdateDecision::Update => {}
    }
    let Some(artifact) = manifest.get_artifact(OS, ARCH, mode) else {
        tracing::warn!("更新清单 {} 未提供当前平台的产物（{OS} / {ARCH} / {mode}）", manifest.version);
        return Ok(None);
    };

    Ok(Some(FoundUpdate {
        info: UpdateInfo {
            version: manifest.version.clone(),
            notes: manifest.release_notes.clone(),
            date: manifest
                .publish_date
                .map(|d| crate::util::time_utils::ISO8601(&d).to_string()),
        },
        severity: manifest.severity,
        artifact,
    }))
}
