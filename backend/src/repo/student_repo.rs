use crate::common::entity::student::Student;
use rbatis::rbatis_codegen::IntoSql;
use rbatis::rbdc::ExecResult;
use rbatis::{py_sql, RBatisTxExecutor};
use rbs::Error;

#[py_sql(
    "
    INSERT INTO students (student_no, name)
    VALUES
    trim ',': for item in students:
        (#{item.student_no}, #{item.name})
"
)]
pub async fn insert(tx: &mut RBatisTxExecutor, students: Vec<Student>) -> Result<ExecResult, Error> {
    impled!()
}

#[py_sql("UPDATE students SET student_no = #{student_no}, name = #{name} WHERE id = #{id}")]
pub async fn update(tx: &mut RBatisTxExecutor, id: i64, student_no: &str, name: &str) -> Result<ExecResult, Error> {
    impled!()
}

#[py_sql("UPDATE students SET name = #{name} WHERE id = #{id}")]
pub async fn update_name(tx: &mut RBatisTxExecutor, id: i64, name: &str) -> Result<ExecResult, Error> {
    impled!()
}

#[py_sql("UPDATE students SET is_deleted = 1 WHERE id IN ${ids.sql()}")]
pub async fn delete(tx: &mut RBatisTxExecutor, ids: Vec<i64>) -> Result<ExecResult, Error> {
    impled!()
}

#[py_sql("UPDATE students SET is_deleted = 0 WHERE id IN ${ids.sql()}")]
pub async fn restore(tx: &mut RBatisTxExecutor, ids: Vec<i64>) -> Result<ExecResult, Error> {
    impled!()
}
