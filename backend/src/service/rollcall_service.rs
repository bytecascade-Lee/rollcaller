use crate::common::entity::record::{Record, RollcallRecord};
use crate::repo::record_repo;
use rbatis::RBatis;

/// 从给定的学生 ID 列表中随机选取一人，写入点名记录并返回完整记录
pub async fn pick_and_save(rb: &RBatis, student_ids: Vec<i64>, session_id: String) -> anyhow::Result<RollcallRecord> {
    // 1. 随机选一个学生
    let idx = rand::random_range(0..student_ids.len());
    let student_id = student_ids[idx];

    // 2. 插入点名记录
    let mut tx = rb.acquire_begin().await?;
    let record = Record::new(student_id, &session_id);
    let result = Record::insert(&mut tx, &record).await?;
    let inserted_id = result
        .last_insert_id
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("Failed to get last insert id"))?;
    tx.commit().await?;

    // 3. 使用 repo 查询完整记录
    let mut tx = rb.acquire_begin().await?;
    let records = record_repo::select_by_ids(&mut tx, vec![inserted_id]).await?;
    tx.commit().await?;
    records
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Inserted record not found after query"))
}
