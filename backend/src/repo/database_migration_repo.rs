use rbatis::rbdc::ExecResult;
use rbatis::{py_sql, RBatis, RBatisTxExecutor};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct MigrationRecord {
    pub version: i64,
    pub checksum: String,
    pub status: String,
}

/// 创建 migration_history 表
pub async fn create_migration_history_table(rb: &RBatis) -> Result<(), rbs::Error> {
    let sql = r#"

CREATE TABLE IF NOT EXISTS migration_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    version INTEGER NOT NULL UNIQUE,
    description TEXT NOT NULL,
    content TEXT NOT NULL,
    checksum TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'RUNNING_IN_TX',
    executed_at INTEGER,
    created_at INTEGER DEFAULT (CAST(UNIXEPOCH('subsec') * 1000 AS INTEGER)),
    updated_at INTEGER DEFAULT (CAST(UNIXEPOCH('subsec') * 1000 AS INTEGER)),
    remark TEXT
);

CREATE TRIGGER IF NOT EXISTS trg_migration_history_after_update
    AFTER UPDATE
    ON migration_history
BEGIN
    UPDATE migration_history
    SET updated_at = CAST(UNIXEPOCH('subsec') * 1000 AS INTEGER)
    WHERE id = NEW.id;
END;

    "#;
    rb.exec(sql, vec![]).await?;
    Ok(())
}

/// 获取已执行的迁移记录
#[py_sql("SELECT version, checksum, status FROM migration_history")]
pub async fn get_existing_history(rb: &RBatis) -> Result<Vec<MigrationRecord>, rbs::Error> {
    impled!()
}

/// 插入 RUNNING 记录
#[py_sql("INSERT INTO migration_history (version, description, content, checksum) VALUES (#{version}, #{description}, #{content}, #{checksum})")]
pub async fn insert_new_history(
    tx: &mut RBatisTxExecutor,
    version: i64,
    description: &str,
    content: &str,
    checksum: &str,
) -> Result<ExecResult, rbs::Error> {
    impled!()
}

/// 更新为 SUCCESS
#[py_sql("UPDATE migration_history SET status = 'SUCCESS', executed_at = #{executed_at}, remark = NULL WHERE version = #{version}")]
pub async fn update_success_status(
    tx: &mut RBatisTxExecutor,
    version: i64,
    executed_at: i64,
) -> Result<ExecResult, rbs::Error> {
    impled!()
}

/// 更新失败记录
#[py_sql("UPDATE migration_history SET status = 'FAILED', remark = #{remark} WHERE version = #{version}")]
pub async fn update_failed_status(
    tx: &mut RBatisTxExecutor,
    version: i64,
    remark: &str,
) -> Result<ExecResult, rbs::Error> {
    impled!()
}
