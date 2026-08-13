use crate::common::entity::attendance_status::AttendanceStatus;
use crate::repo::attendance_status_repo;
use rbatis::RBatis;
use rbs::value;
pub async fn create(rb: &RBatis, attendance_status: AttendanceStatus) -> anyhow::Result<()> {
    AttendanceStatus::insert(rb, &attendance_status).await?;
    Ok(())
}

pub async fn update(rb: &RBatis, attendance_status: AttendanceStatus) -> anyhow::Result<()> {
    let mut tx = rb.acquire_begin().await?;
    attendance_status_repo::update(
        &mut tx,
        attendance_status.id,
        &*attendance_status.name,
        &*attendance_status.background,
        &*attendance_status.color,
        attendance_status.remark.as_deref(),
    )
    .await?;
    Ok(())
}

pub async fn get_all(rb: &RBatis) -> anyhow::Result<Vec<AttendanceStatus>> {
    Ok(AttendanceStatus::select_by_map(rb, value! {}).await?)
}
