use crate::common::entity::record::RollcallRecord;
use crate::repo::record_repo;
use anyhow::anyhow;
use rbatis::RBatis;

/// 查询最近 100 条点名记录（含学生姓名、学号）
pub async fn list_all(rb: &RBatis) -> anyhow::Result<Vec<RollcallRecord>> {
    let mut tx = rb.acquire_begin().await?;
    let records = record_repo::select_all(&mut tx).await?;
    tx.commit().await?;
    Ok(records)
}

pub async fn update_attendance_status(
    rb: &RBatis,
    ids: Vec<i64>,
    attendance_status: i8,
) -> anyhow::Result<Vec<RollcallRecord>> {
    let mut tx = rb.acquire_begin().await?;
    let result = record_repo::update_attendance_status(&mut tx, &ids, attendance_status).await?;
    let len = ids.len() as u64;
    if result.rows_affected != len {
        tx.rollback().await?;
        return Err(anyhow!("影响行不正确：应为{len}，实为{0}，已回滚", result.rows_affected));
    }
    tx.commit().await?;
    let mut select_tx = rb.acquire_begin().await?;
    select_tx.commit().await?;
    Ok(record_repo::select_by_ids(&mut select_tx, ids).await?)
}

pub async fn update_remark(rb: &RBatis, ids: Vec<i64>, remark: String) -> anyhow::Result<Vec<RollcallRecord>> {
    let mut tx = rb.acquire_begin().await?;
    let result = record_repo::update_remark(&mut tx, &ids, &remark).await?;
    let len = ids.len() as u64;
    if result.rows_affected != len {
        tx.rollback().await?;
        return Err(anyhow!("影响行不正确：应为{len}，实为{0}，已回滚", result.rows_affected));
    }
    tx.commit().await?;
    let mut select_tx = rb.acquire_begin().await?;
    select_tx.commit().await?;
    Ok(record_repo::select_by_ids(&mut select_tx, ids).await?)
}
