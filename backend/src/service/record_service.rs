use crate::common::entity::record::RollcallRecord;
use crate::repo::record_repo;
use rbatis::RBatis;

/// 查询最近 100 条点名记录（含学生姓名、学号）
pub async fn list_all(rb: &RBatis) -> anyhow::Result<Vec<RollcallRecord>> {
    let mut tx = rb.acquire_begin().await?;
    let records = record_repo::select_all(&mut tx).await?;
    tx.commit().await?;
    Ok(records)
}
