use tracing::debug;

/// 从给定的学生 ID 列表中随机选取一人
pub fn pick(student_ids: Vec<i64>) -> anyhow::Result<i64> {
    let candidates = student_ids.len();
    let picked = student_ids[rand::random_range(0..candidates)];
    // 一节课会点几十次，只能 DEBUG；`candidates` 用于分辨"点错了"还是"名单本身不对"
    debug!(picked, candidates, "点名");
    Ok(picked)
}
