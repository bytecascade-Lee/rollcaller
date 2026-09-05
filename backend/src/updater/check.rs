//! 更新检查：拉版本索引（versions.json）挑"最高可安装版本" → 决策目标 → 拉该版本清单（本地缓存优先）→ 判定 → 返回更新信息
//!
//! # versions.json 在 check 流程里的职责
//!
//! 版本索引随每次发布附带全部历史版本的 severity/force。check 据此在 `(current, latest]`
//! 区间内**从高到低**扫描，取首个被 [`evaluate`] 放行的版本作为目标版本。
//! - 例：latest 0.4.6 是 normal 补丁被 Minor 门槛挡下时，0.4.5 important 通过豁免入选。
//!
//! # 判定语义分层
//!
//! - 纯策略判定在 `service/update/version.rs::decide`（不认识 severity，保持纯粹"通道门禁 + 幅度门槛"语义）；
//! - severity 豁免与 force 合法性在 [`evaluate`] 融合（本模块 check 与 download 入口复核共用同一个纯函数，判定语义只写一次）；
//! - 索引 ↔ 清单的一致性复判暂在本模块实现（见 [`validate_manifest`]）。
//!   该职责的归宿是version.rs——后期在 version 中新增函数统一处理 severity/force 后迁走。
//!
//! # 数据获取与缓存
//!
//! - `versions.json` 每次发布都会新增条目，缓存无意义，**每次检查都拉取**
//!   `releases/latest/download/versions.json`（URL 见 [`versions_index_url`]）；
//! - 目标版本的清单 `latest-{github|cnb}.json` 以固定文件名挂在**对应版本**的 Release 下
//!   （URL 模板见 [`latest_manifest_url`]，[`PLACEHOLDER`] 占位替换），拉取后落盘缓存
//!   `cache_dir/update/{source}/{version}.json`（按版本号命名、不含 v）。下次检查再次选中同一
//!   目标版本时直接读缓存，不再走网络；versions.json 仍照常拉取以感知新版本。
//! - 起始版本语义：版本索引/清单/签名自 `common::constant::update::*_FILE_START_*`
//!   标定的版本起才存在，更早的历史版本在 GitHub/CNB 上均无对应文件。由于目标版本恒
//!   不低于起始版本（索引候选均在其上），本流程无需对起始版本做特判。
//!

use crate::common::constant::sys::{ARCH, OS};
use crate::common::constant::update::{
    PLACEHOLDER, SPECIFIED_LATEST_MANIFEST_CNB, SPECIFIED_LATEST_MANIFEST_GITHUB, VERSIONS_INDEX_CNB, VERSIONS_INDEX_GITHUB,
};
use crate::common::entity::update::{Artifact, HistoryVersion, Policy, UpdateManifest};
use crate::common::enums::update::{Severity, UpdateDecision, UpdateLevel, UpdateSource};
use crate::config::app_paths::AppMode;
use crate::service::update::version::decide;
use anyhow::{anyhow, Context};
use reqwest::Client;
use semver::Version;
use serde::Serialize;
use std::fmt;
use std::path::{Path, PathBuf};
use ts_rs::TS;

/// 更新信息
///
/// `severity` / `force` 为清单字段的透传：前端据此决定徽标与"是否提供忽略/稍后"的交互，真正的强制安装流程属于后续 force 任务。
#[derive(Debug, Clone, Serialize, TS)]
pub struct UpdateInfo {
    #[ts(type = "string")]
    pub version: Version,
    /// releaseNotes
    pub notes: Option<String>,
    /// publishDate（格式化后的字符串，供前端展示）
    pub date: Option<String>,
    /// 本次要下载的产物（nsis 或 portable 之一）
    pub artifact: Artifact,
    /// 更新严重程度（默认 normal；normal 之外的版本已豁免幅度门槛才会走到这里）
    pub severity: Severity,
    /// 是否强制（默认 false；仅 critical 合法）
    pub force: bool,
}

/// 一次更新检查的对外结果（service 层组装，供命令层/前端区分四态）
///
/// 前端可按类型区分三种情形：`NoUpdate`（含"策略不符 / 当前形态无产物"，不打扰）、
/// `Available`（展示更新信息）、网络/清单失败（提示"检查失败"而非"无更新"）。
/// 四态语义尚不完善，后续重构——届时只调整本类型与 service 边界。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", content = "data")]
pub enum CheckOutcome {
    /// 无可用更新（无更高版本 / 策略不符 / 当前运行形态无对应产物）
    NoUpdate,
    /// 有可用更新，完整信息见 `info`
    Available(UpdateInfo),
    /// 网络层失败：连不上 / HTTP 错误 / 响应读取失败
    NetworkError(String),
    /// 版本索引或清单内容非法（JSON 格式 / 字段冲突 / 索引与清单不一致）
    InvalidManifest(String),
}

/// 版本索引地址：`releases/latest/download/versions.json`
fn versions_index_url(source: UpdateSource) -> &'static str {
    match source {
        UpdateSource::Github => VERSIONS_INDEX_GITHUB,
        UpdateSource::CNB => VERSIONS_INDEX_CNB,
    }
}

/// 指定版本 Release 的清单地址
fn latest_manifest_url(source: UpdateSource, version: &Version) -> String {
    match source {
        UpdateSource::Github => SPECIFIED_LATEST_MANIFEST_GITHUB,
        UpdateSource::CNB => SPECIFIED_LATEST_MANIFEST_CNB,
    }
        .replace(PLACEHOLDER, &version.to_string())
}

/// 目标版本清单的本地缓存路径：`cache_dir/update/{source}/{version}.json`
fn manifest_cache_path(cache_dir: &Path, source: UpdateSource, version: &Version) -> PathBuf {
    cache_dir
        .join("update")
        .join(source.to_string().to_ascii_lowercase())
        .join(format!("{version}.json"))
}

/// 读缓存文件；不存在或读取失败返回 `None`
fn read_cached(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// 写缓存文件（自动创建父目录）；失败由调用方决定是否阻断
fn write_cached(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)
}

/// 拉取远程文本
async fn fetch_json_text(client: &Client, url: &str) -> anyhow::Result<String> {
    let response = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .context(anyhow!("检查更新失败：网络错误"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("检查更新失败：{url}服务器返回 HTTP {status}"));
    }
    response
        .text()
        .await
        .context(anyhow!("检查更新失败：读取响应失败"))
}

/// 解析版本索引 JSON 为候选列表（顺序与合法性由调用方把关）
fn parse_index(text: &str) -> anyhow::Result<Vec<HistoryVersion>> {
    serde_json::from_str(text).context(anyhow!("版本索引不是合法 JSON"))
}

/// 决策目标版本：在比当前新的候选中从高到低扫描，取首个被放行的版本。
///
/// 候选过滤（`> current`）与倒序排序也在此完成；无任何版本放行时返回 `Ok(None)`。
fn pick_target(current: &Version, candidates: &[HistoryVersion], policy: &Policy) -> anyhow::Result<Option<HistoryVersion>> {
    let mut newer: Vec<&HistoryVersion> = candidates.iter().filter(|hv| hv.version > *current).collect();
    newer.sort_by(|a, b| b.version.cmp(&a.version));
    for hv in newer {
        match evaluate(current, &hv.version, policy, hv.severity, hv.force) {
            Ok(UpdateDecision::Update) => return Ok(Some(hv.clone())),
            Ok(UpdateDecision::Skip) => continue,
            Err(e) => return Err(anyhow!("版本索引不合法（{}）：{e}", hv.version)),
        }
    }
    Ok(None)
}

/// 解析目标版本清单 JSON，返回结构体与原始 JSON
fn parse_manifest(text: &str) -> anyhow::Result<UpdateManifest> {
    serde_json::from_str(text).context(anyhow!("清单格式不符合规范"))
}

/// 一致性复判 + 取产物 + 组装更新信息。
///
/// 索引只做预筛选，清单才是该版本的最终发布数据：版本号必须与索引一致，且以**清单
/// 自身**的 severity/force 复判一次；不一致视为发布端错误（宁可暴露也不静默降级）。
/// 当前运行形态无对应产物时返回 `Ok(None)`（无更新，不打扰用户）。
fn validate_manifest(
    current: &Version,
    policy: &Policy,
    expected: &Version,
    manifest: &UpdateManifest,
    mode: AppMode,
) -> anyhow::Result<Option<UpdateInfo>> {
    if manifest.version != *expected {
        return Err(anyhow!(
            "版本索引与清单不一致：索引指向 {expected}，清单返回 {}",
            manifest.version
        ));
    }
    match evaluate(current, &manifest.version, policy, manifest.severity, manifest.force) {
        Err(e) => return Err(anyhow!("清单不合法（{}）：{e}", manifest.version)),
        Ok(UpdateDecision::Skip) => {
            return Err(anyhow!(
                "版本索引与清单不一致：{} 的清单标注的 severity/force 不足以放行本次更新",
                manifest.version
            ))
        }
        Ok(UpdateDecision::Update) => {}
    }
    let Some(artifact) = manifest.get_artifact(OS, ARCH, mode) else {
        return Ok(None);
    };

    Ok(Some(UpdateInfo {
        version: manifest.version.clone(),
        notes: manifest.release_notes.clone(),
        date: manifest.publish_date.map(|d| d.to_string()),
        artifact,
        severity: manifest.severity,
        force: manifest.force,
    }))
}

/// 检查是否有可用更新
///
/// # 参数
/// - `current`：当前版本，由调用方从 `app_info().version` 解析后传入；
/// - `policy`：当前用户策略（设置存储落地前由调用方提供默认值）；
/// - `source`：拉 GitHub 还是 CNB（URL 统一取自 common 常量并由本模块映射）；
/// - `mode`：运行形态（Develop 短路不发起请求；Install/Portable 决定取哪类产物）；
/// - `cache_dir`：缓存根目录（目标版本清单按版本号落盘于此，避免重复拉取）。
///
/// # 返回
/// - `anyhow::Ok(Some(info))`：有可用更新；
/// - `anyhow::Ok(None)`：无更新（无更高版本 / 策略不符 / 当前形态无产物 / 开发模式）；
/// - `anyhow::Err`：检查失败（网络或清单非法）。
pub async fn check(
    client: &Client,
    source: UpdateSource,
    current: &Version,
    policy: &Policy,
    mode: AppMode,
    cache_dir: &Path,
) -> anyhow::Result<Option<UpdateInfo>> {
    // 开发模式不更新（不发起任何网络请求）
    if matches!(mode, AppMode::Develop) {
        return Ok(None);
    }

    // 1. 拉版本索引（不缓存：versions.json 每次发布都有新增）
    let index_text = fetch_json_text(client, versions_index_url(source)).await?;
    let candidates = parse_index(&index_text)?;

    // 2. 决策目标版本；无放行版本 → 无更新
    let Some(target) = pick_target(current, &candidates, policy)? else {
        return Ok(None);
    };

    // 3. 目标版本清单：缓存优先（命中则免网络）；缺失或缓存损坏则拉取并写缓存
    let cache_path = manifest_cache_path(cache_dir, source, &target.version);
    let manifest = match read_cached(&cache_path).and_then(|t| parse_manifest(&t).ok()) {
        Some(pair) => pair,
        None => {
            // 缓存存在但解析失败 → 视为损坏，清除后回源
            if read_cached(&cache_path).is_some() {
                let _ = std::fs::remove_file(&cache_path);
            }
            let text = fetch_json_text(client, &latest_manifest_url(source, &target.version)).await?;
            if let Err(e) = write_cached(&cache_path, &text) {
                tracing::warn!("写入更新清单缓存失败（{}）：{e}", cache_path.display());
            }
            parse_manifest(&text)?
        }
    };

    // 4. 一致性复判 + 按运行形态取产物；无产物视为无更新
    validate_manifest(current, policy, &target.version, &manifest, mode)
}

/// 版本判定 + severity/force 融合
///
/// # severity 豁免语义
///
/// | decide 返回 Skip 的原因 | normal | important / critical & force=false | critical & force=true |
/// |---|---|---|---|
/// | 幅度不足（被 Minor/Major 门槛挡） | 不通知 | **豁免 → 通知** | 豁免 → 通知 |
/// | 用户关闭更新（level=None） | 不通知 | 不通知 | **穿透 → 通知** |
/// | Stable 通道拦预发布 | 不豁免 | 不豁免 | 不豁免 |
/// | 同版本 / 降级 | 不豁免 | 不豁免 | 不豁免 |
///
/// 实现方式：豁免 = 仅把幅度门槛临时降到 `Patch`（等价"无视幅度门槛"），其余
/// 门禁原样交给 `decide`——因此无需给 `decide` 增加 Skip 原因，也不破坏其纯
/// Copy 枚举形态。`force=true` 且 `severity != Critical` 视为清单非法（Err），
/// 宁可暴露发布端错误也不静默降级。
pub fn evaluate(
    current: &Version,
    latest: &Version,
    policy: &Policy,
    severity: Severity,
    force: bool,
) -> Result<UpdateDecision, String> {
    // 0. force 合法性：仅 critical 可强制（发布端错误，check 阶段直接报错）
    if force && severity != Severity::Critical {
        return Err(format!(
            "清单不合法：force=true 仅允许 severity=critical（当前 {severity:?}）"
        ));
    }
    // 用户关闭更新（level=UpdateLevel::Never）：normal / important / 非强制的 critical 均不打扰；
    // 唯一例外 = critical + force（发布方明确"必须更新"）
    if policy.level == UpdateLevel::Never {
        return Ok(if severity == Severity::Critical && force {
            UpdateDecision::Update
        } else {
            UpdateDecision::Skip
        });
    }
    // severity 豁免：normal 之外把幅度门槛降到 Patch，等价"无视幅度门槛"；
    // 通道门禁 / 同版本 / 降级 / 逃逸 / 递进由 decide 原样保留
    let level = if severity == Severity::Normal {
        policy.level
    } else {
        UpdateLevel::Patch
    };
    Ok(decide(current, latest, level, policy.channel))
}
