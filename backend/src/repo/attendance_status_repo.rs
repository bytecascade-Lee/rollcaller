use rbatis::rbatis_codegen::IntoSql;
use rbatis::rbdc::ExecResult;
use rbatis::{py_sql, RBatisTxExecutor};
use rbs::Error;

#[py_sql(
    "
UPDATE attendance_status_definition
SET name = #{name},
    background = #{background},
    color = #{color},
    remark = #{remark}
WHERE id = #{id}
"
)]
pub async fn update(
    tx: &mut RBatisTxExecutor,
    id: i8,
    name: &str,
    background: &str,
    color: &str,
    remark: Option<&str>,
) -> Result<ExecResult, Error> {
    impled!()
}

#[py_sql("UPDATE attendance_status_definition SET is_deleted = 1 WHERE id IN ${ids.sql()}")]
pub async fn delete(tx: &mut RBatisTxExecutor, id: Vec<i64>) -> Result<ExecResult, Error> {
    impled!()
}

#[py_sql("UPDATE attendance_status_definition SET is_deleted = 0 WHERE id IN ${ids.sql()}")]
pub async fn restore(tx: &mut RBatisTxExecutor, id: Vec<i64>) -> Result<ExecResult, Error> {
    impled!()
}
