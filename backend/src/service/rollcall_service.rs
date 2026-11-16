use anyhow::Context;
use rand::seq::IndexedRandom;
use tracing::debug;

/// 从给定的学生 ID 列表中随机选取一人
pub fn pick(student_ids: &[i64]) -> anyhow::Result<i64> {
    let picked = *student_ids.choose(&mut rand::rng()).context("学生名单为空，无法点名")?;
    debug!(picked, candidates = student_ids.len(), "点名");
    Ok(picked)
}
