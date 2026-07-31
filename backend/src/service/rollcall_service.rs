/// 从给定的学生 ID 列表中随机选取一人
pub fn pick(student_ids: Vec<i64>) -> anyhow::Result<i64> {
    Ok(student_ids[rand::random_range(0..student_ids.len())])
}
