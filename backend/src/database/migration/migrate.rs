//! 迁移账本 `database_migration_history` 与 `app_meta` 的 SQL

use rbatis::rbdc::ExecResult;
use rbatis::{py_sql, RBatis, RBatisTxExecutor};
use serde::Deserialize;

/// 旧账本表名
pub const LEGACY_TABLE: &str = "migration_history";
/// 新账本表名
pub const HISTORY_TABLE: &str = "database_migration_history";
/// 新账本表的建表语句
pub const CREATE_HISTORY_SQL: &str = "CREATE TABLE IF NOT EXISTS database_migration_history (
         version    INTEGER PRIMARY KEY,
         applied_at INTEGER NOT NULL
     )";

/// `app_meta` 的建表语句
pub const CREATE_META_SQL: &str = "CREATE TABLE IF NOT EXISTS app_meta (
         key   TEXT PRIMARY KEY,
         value TEXT NOT NULL
     )";

/// 已应用的一条迁移
#[derive(Deserialize)]
pub struct AppliedMigration {
    pub version: i64,
}

/// 表是否存在（1 = 存在）
#[py_sql("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = #{name}")]
pub async fn table_exists(rb: &RBatis, name: &str) -> Result<i64, rbs::Error> {
    impled!()
}

/// 表里是否含某一列（1 = 含）
#[py_sql("SELECT COUNT(*) FROM pragma_table_info(#{table}) WHERE name = #{column}")]
pub async fn column_exists(rb: &RBatis, table: &str, column: &str) -> Result<i64, rbs::Error> {
    impled!()
}

/// 旧账本表的行数
#[py_sql("SELECT COUNT(*) FROM migration_history")]
pub async fn count_legacy_rows(rb: &RBatis) -> Result<i64, rbs::Error> {
    impled!()
}

/// 新账本表的已应用版本（升序）
#[py_sql("SELECT version FROM database_migration_history ORDER BY version")]
pub async fn select_applied(rb: &RBatis) -> Result<Vec<AppliedMigration>, rbs::Error> {
    impled!()
}

/// 记录一条已应用的迁移；**必须与迁移 SQL 处在同一个事务里**
#[py_sql("INSERT INTO database_migration_history (version, applied_at) VALUES (#{version}, #{applied_at})")]
pub async fn insert_applied(tx: &mut RBatisTxExecutor, version: i64, applied_at: i64) -> Result<ExecResult, rbs::Error> {
    impled!()
}

/// 把旧账本的版本集合搬进新表（一次性收敛用）
#[py_sql(
    "INSERT INTO database_migration_history (version, applied_at)
     SELECT version, COALESCE(executed_at, created_at, 0) FROM migration_history"
)]
pub async fn copy_legacy_rows(tx: &mut RBatisTxExecutor) -> Result<ExecResult, rbs::Error> {
    impled!()
}

/// 外键违例数量；`pragma_foreign_key_check` 的表值函数形式，可在事务内执行
#[py_sql("SELECT COUNT(*) FROM pragma_foreign_key_check")]
pub async fn foreign_key_violations(tx: &mut RBatisTxExecutor) -> Result<i64, rbs::Error> {
    impled!()
}

/// 新账本表的行数（收敛时校验用，在事务内执行）
#[py_sql("SELECT COUNT(*) FROM database_migration_history")]
pub async fn count_history_rows(tx: &mut RBatisTxExecutor) -> Result<i64, rbs::Error> {
    impled!()
}

/// 建新账本表
pub async fn create_history_table(rb: &RBatis) -> Result<(), rbs::Error> {
    rb.exec(CREATE_HISTORY_SQL, vec![]).await?;
    Ok(())
}

/// 建应用元数据表（形状永不变化，只加行不加列）
pub async fn create_meta_table(rb: &RBatis) -> Result<(), rbs::Error> {
    rb.exec(CREATE_META_SQL, vec![]).await?;
    Ok(())
}

/// 写一条元数据（存在则覆盖）
#[py_sql(
    "
INSERT INTO app_meta (key, value) VALUES (#{key}, #{value})
ON CONFLICT(key) DO UPDATE SET value = excluded.value
"
)]
pub async fn upsert_meta(rb: &RBatis, key: &str, value: &str) -> Result<ExecResult, rbs::Error> {
    impled!()
}
