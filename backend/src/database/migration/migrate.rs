//! 账本 `database_migration_history`、元数据 `app_meta` 与迁移期外键校验的 SQL

use rbatis::rbdc::ExecResult;
use rbatis::{py_sql, RBatis, RBatisTxExecutor};
use serde::Deserialize;

/// 旧账本表名
pub const LEGACY_TABLE: &str = "migration_history";
/// 新账本表名
pub const HISTORY_TABLE: &str = "database_migration_history";
/// 新账本表的建表语句
///
/// `migrated_at` 是规范 ISO 8601 文本（UTC、恒三位小数）。历史形状是 `INTEGER` 毫秒，
/// 由 [`REBUILD_STAMP_SQL`] 就地改写。
pub const CREATE_HISTORY_SQL: &str = "CREATE TABLE IF NOT EXISTS database_migration_history (
         version    INTEGER PRIMARY KEY,
         migrated_at TEXT NOT NULL
     )";

/// 账本时间戳列从 `INTEGER` 毫秒改写为 ISO 8601 文本
///
/// 账本不能用迁移改自己，所以走启动时的幂等收敛（见 `ledger.rs`）。
/// `CASE` 是给"列声明类型为空、值可能已是文本"的库兜底：只有确认不是 ISO 串才按毫秒换算，
/// 换算不出来的值（`strftime` 返回 NULL）会被 `NOT NULL` 拦住并整批回滚，不会写脏数据。
pub const REBUILD_STAMP_SQL: &str = "
    CREATE TABLE database_migration_history_stamp_new (
        version    INTEGER PRIMARY KEY,
        migrated_at TEXT NOT NULL
    );
    INSERT INTO database_migration_history_stamp_new (version, migrated_at)
    SELECT version,
           CASE
               WHEN typeof(migrated_at) = 'text' AND migrated_at LIKE '____-__-__T__:__:__%'
                   THEN migrated_at
               ELSE strftime('%Y-%m-%dT%H:%M:%fZ', migrated_at / 1000.0, 'unixepoch')
           END
    FROM database_migration_history;
    DROP TABLE database_migration_history;
    ALTER TABLE database_migration_history_stamp_new RENAME TO database_migration_history;
";

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

/// 某一列的声明类型（`pragma_table_info` 的 `type`；未声明类型时为空串）
///
/// 调用前必须确认列存在，否则查不到行会解码失败。
#[py_sql("SELECT type FROM pragma_table_info(#{table}) WHERE name = #{column}")]
pub async fn column_type(rb: &RBatis, table: &str, column: &str) -> Result<String, rbs::Error> {
    impled!()
}

/// 旧账本表的行数
#[py_sql("SELECT COUNT(*) FROM migration_history")]
pub async fn count_legacy_rows(rb: &RBatis) -> Result<i64, rbs::Error> {
    impled!()
}

/// 新账本表的行数（形状改写前后对账用）
#[py_sql("SELECT COUNT(*) FROM database_migration_history")]
pub async fn count_history_rows(rb: &RBatis) -> Result<i64, rbs::Error> {
    impled!()
}

/// 新账本表的已应用版本（升序）
#[py_sql("SELECT version FROM database_migration_history ORDER BY version")]
pub async fn select_applied(rb: &RBatis) -> Result<Vec<AppliedMigration>, rbs::Error> {
    impled!()
}

/// 记录一条已应用的迁移；**必须与迁移 SQL 处在同一个事务里**
#[py_sql("INSERT INTO database_migration_history (version, migrated_at) VALUES (#{version}, #{migrated_at})")]
pub async fn insert_applied(tx: &mut RBatisTxExecutor, version: i64, migrated_at: &str) -> Result<ExecResult, rbs::Error> {
    impled!()
}

/// 把旧账本的版本集合搬进新表（一次性收敛用）；`rows_affected` 即搬迁行数
///
/// 旧表的时间列是 `INTEGER` 毫秒、且可能为 `NULL`。`COALESCE(..., 0)` 在 TEXT 列下会写出
/// 非法的 `"0"`，所以缺值要显式换成纪元时刻。
#[py_sql(
    "INSERT INTO database_migration_history (version, migrated_at)
     SELECT version,
            CASE
                WHEN COALESCE(executed_at, created_at) IS NULL
                    THEN strftime('%Y-%m-%dT%H:%M:%fZ', 0, 'unixepoch')
                ELSE strftime('%Y-%m-%dT%H:%M:%fZ', COALESCE(executed_at, created_at) / 1000.0, 'unixepoch')
            END
     FROM migration_history"
)]
pub async fn copy_legacy_rows(tx: &mut RBatisTxExecutor) -> Result<ExecResult, rbs::Error> {
    impled!()
}

/// 外键违例数量；`pragma_foreign_key_check` 的表值函数形式，可在事务内执行
#[py_sql("SELECT COUNT(*) FROM pragma_foreign_key_check")]
pub async fn foreign_key_violations(tx: &mut RBatisTxExecutor) -> Result<i64, rbs::Error> {
    impled!()
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
