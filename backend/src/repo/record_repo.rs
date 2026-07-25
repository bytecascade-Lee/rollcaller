use crate::common::entity::record::RollcallRecord;
use rbatis::rbatis_codegen::IntoSql;
use rbatis::{py_sql, RBatisTxExecutor};

#[py_sql(
    "
SELECT
    rc.id,
    st.id AS student_id,
    st.student_no,
    st.name,
    rc.attendance_status,
    rc.remark,
    rc.rollcall_at,
    rc.session_id
FROM
    records AS rc
JOIN students AS st ON st.id = rc.student_id
WHERE
    rc.is_deleted = 0
    AND st.is_deleted = 0
ORDER BY
    rc.rollcall_at DESC
LIMIT
    100
"
)]
pub async fn select_all(tx: &mut RBatisTxExecutor) -> Result<Vec<RollcallRecord>, rbatis::Error> {
    impled!()
}

#[py_sql(
    "
SELECT
    rc.id,
    st.id AS student_id,
    st.student_no,
    st.name,
    rc.attendance_status,
    rc.remark,
    rc.rollcall_at,
    rc.session_id
FROM
    records AS rc
JOIN students AS st ON st.id = rc.student_id
WHERE
    rc.id IN ${ids.sql()}
    AND rc.is_deleted = 0
    AND st.is_deleted = 0
"
)]
pub async fn select_by_ids(tx: &mut RBatisTxExecutor, ids: Vec<i64>) -> Result<Vec<RollcallRecord>, rbatis::Error> {
    impled!()
}
