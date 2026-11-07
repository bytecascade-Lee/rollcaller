use crate::updater::decide;
use crate::updater::model::{HistoryVersion, Policy, UpdateDecision};
use anyhow::{anyhow, Context};
use semver::Version;

/// 解析版本索引 JSON 为候选列表（顺序与合法性由调用方把关）
pub fn parse_index(text: &str) -> anyhow::Result<Vec<HistoryVersion>> {
    serde_json::from_str(text).context(anyhow!("版本索引不是合法 JSON"))
}

/// 决策目标版本：在比当前新的候选中从高到低扫描，取首个被放行的版本。
///
/// 候选过滤（`> current`）与倒序排序也在此完成；无任何版本放行时返回 `Ok(None)`。
pub fn pick_target(current: &Version, candidates: &[HistoryVersion], policy: &Policy) -> anyhow::Result<Option<HistoryVersion>> {
    let mut newer: Vec<&HistoryVersion> = candidates.iter().filter(|hv| hv.version > *current).collect();
    newer.sort_by(|a, b| b.version.cmp(&a.version));
    for hv in newer {
        match decide::evaluate(current, &hv.version, policy, hv.severity) {
            UpdateDecision::Update => return Ok(Some(hv.clone())),
            UpdateDecision::Skip => continue,
        }
    }
    Ok(None)
}
