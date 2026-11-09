use crate::updater::decide;
use crate::updater::model::{HistoryVersion, Policy};
use anyhow::{anyhow, Context};
use semver::Version;

/// 解析版本索引 JSON 为候选列表
pub fn parse_index(text: &str) -> anyhow::Result<Vec<HistoryVersion>> {
    serde_json::from_str(text).context(anyhow!("版本索引不是合法 JSON"))
}

/// 决策目标版本：在比当前新的候选中从高到低扫描，取首个被放行的版本
pub fn pick_target(
    current: &Version,
    candidates: &[HistoryVersion],
    policy: &Policy,
) -> anyhow::Result<Option<HistoryVersion>> {
    let mut newer: Vec<&HistoryVersion> = candidates.iter().filter(|hv| hv.version > *current).collect();
    newer.sort_by(|a, b| b.version.cmp(&a.version));
    Ok(newer
        .into_iter()
        .find(|hv| decide::decide(current, &hv.version, policy, hv.severity).is_update())
        .cloned())
}
