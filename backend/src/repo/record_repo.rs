use crate::common::entity::record::RollcallRecord;
use rbatis::{py_sql, RBatisTxExecutor};

#[py_sql("
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
")]
pub async fn select_rollcall_record(tx: &mut RBatisTxExecutor) -> Result<Vec<RollcallRecord>, rbatis::Error> {
    impled!()
}
