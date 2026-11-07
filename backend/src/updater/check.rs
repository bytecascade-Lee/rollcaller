//! 更新检查：拉版本索引（versions.json）挑"最高可安装版本" → 决策目标 → 拉该版本清单（本地缓存优先）→ 判定 → 返回更新信息
//!
//! # 判定语义分层
//!
//! - 纯策略判定在 `updater/decide.rs::decide`（不认识 severity，保持纯粹"通道门禁 + 幅度门槛"语义）；
//! - severity 豁免与 critical 强制在 [`decide::evaluate`] 融合（本模块 check 与 download 入口复核共用同一个纯函数，判定语义只写一次）；
//! - 索引 ↔ 清单的一致性复判在本模块 [`validate_manifest`]——清单才是该版本的最终发布数据。
//!
//! # 数据获取与缓存
//!
//! - `versions.json` 每次发布都会新增条目，缓存无意义，**每次检查都拉取**
//!   （URL 见 [`versions_index_url`]）；
//! - 目标版本的清单 `latest-{github|cnb|develop}.json` 以固定文件名挂在**对应版本**的 Release 下
//!   （URL 见 [`manifest_url`]），拉取后落盘缓存 `cache_dir/update/{source}/{version}.json`
//!   （按版本号命名、不含 v）。下次检查再次选中同一目标版本时直接读缓存，不再走网络；
//!   versions.json 仍照常拉取以感知新版本。
//!   ⚠️ 当前缓存只判"能否解析成 JSON"，不判"是不是这一版该有的那份字节"——内容合法但事实错误的
//!   缓存（如 0.8.4 的 CNB 清单少了 `/-/`）永远不会被推翻，发布端覆盖上传也修不回来。
//!   清单摘要校验（摘要写进 `versions.json`、缓存不符即删重拉）尚未实施。
//!
//! # `versions.json` 在 [`check`] 流程里的职责
//!
//! 版本索引随每次发布附带全部历史版本的 severity。
//! [`check`] 据此在 `(current, latest]` 区间内**从高到低**扫描，取首个被 [`decide::evaluate`] 放行的版本作为目标版本。
//! - 例：latest 0.4.6 是 normal 补丁，被 Minor 门槛挡下时，0.4.5 important 通过豁免入选。
//!

use crate::common::constant::sys::{ARCH, OS};
use crate::config::app_paths::AppMode;
use crate::updater::model::{FoundUpdate, Policy, UpdateDecision, UpdateInfo, UpdateManifest, UpdateSource};
use crate::updater::remote::{fetch, index, manifest};
use crate::updater::urls::{manifest_url, versions_index_url};
use crate::updater::{decide, paths};
use anyhow::anyhow;
use reqwest::Client;
use semver::Version;
use std::fs;

/// 检查是否有可用更新
///
/// # 参数
/// - `current`：当前版本，由调用方从 [`crate::config::app_info::AppInfo`] 的 `version` 字段解析后传入；
/// - `policy`：当前用户策略，目前固定为 [`Policy::default`]，后期开放设置后可以让用户选择；
/// - `source`：源，目前固定为 [`UpdateSource::CNB`]，后期开放设置后可以让用户选择；
/// - `mode`：运行模式。
///
/// 目标版本清单的缓存路径由 [`paths::manifest`] 统一给出（按源 + 版本寻址，落 app 缓存目录），本领域不再自行拼装。
///
/// # 返回
/// - `anyhow::Ok(Some(found))`：命中目标更新（展示信息 + 下载凭据，见 [`FoundUpdate`]），
///   service 层拆包：凭据与展示信息一并存入会话，展示信息组装对外结果；
/// - `anyhow::Ok(None)`：无更新（无更高版本 / 策略不符 / 当前形态无产物）；
/// - `anyhow::Err`：检查失败（网络或清单非法）。
pub async fn check(
    client: &Client,
    source: UpdateSource,
    current: &Version,
    policy: &Policy,
    mode: AppMode,
) -> anyhow::Result<Option<FoundUpdate>> {
    // 1. 拉版本索引；versions.json 每次发布都有新增，不缓存
    let index_text = fetch::fetch_json_text(client, &versions_index_url(source)).await?;
    let candidates = index::parse_index(&index_text)?;

    // 2. 决策目标版本；无放行版本 → 无更新
    let Some(target) = index::pick_target(current, &candidates, policy)? else {
        return Ok(None);
    };

    // 3. 目标版本清单：缓存优先（命中则免网络）；缺失或缓存损坏则拉取并写缓存
    let cache_path = paths::manifest(&source, &target.version);
    let cache_content = fs::read_to_string(&cache_path).ok();
    let manifest = match cache_content.clone().and_then(|t| manifest::parse_manifest(&t).ok()) {
        Some(pair) => pair,
        None => {
            // 缓存存在但解析失败 → 视为损坏，清除后回源
            if cache_content.is_some() {
                let _ = fs::remove_file(&cache_path);
            }
            let text = fetch::fetch_json_text(client, &manifest_url(source, &target.version)).await?;
            // 避免父目录不存在
            if let Some(parent) = cache_path.parent() {
                fs::create_dir_all(parent)?;
            }
            if let Err(e) = fs::write(&cache_path, &text) {
                tracing::warn!("写入更新清单缓存失败（{}）：{e}", cache_path.display());
            }
            manifest::parse_manifest(&text)?
        }
    };

    // 4. 一致性复判 + 按运行形态取产物；无产物视为无更新
    validate_manifest(current, policy, &target.version, &manifest, mode)
}

/// 一致性复判 + 取产物 + 组装检查命中。
///
/// 索引只做预筛选，清单才是该版本的最终发布数据：版本号必须与索引一致，且以**清单
/// 自身**的 severity 复判一次；不一致视为发布端错误（宁可暴露也不静默降级）。
/// 当前运行形态无对应产物时返回 `Ok(None)`（无更新，不打扰用户），
/// 并留一条 warn 日志——「发布端漏了矩阵里的某个 target」只能靠这条痕迹排查。
fn validate_manifest(
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
