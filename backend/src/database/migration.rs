//! 数据库结构迁移
//!
//! 启动顺序：`app_meta` -> 账本收敛 -> 读已应用集合 -> 单事务执行待执行迁移

mod ledger;
mod migrate;
mod snapshot;
mod sql;

use crate::util::time_utils::current_iso_8601;
use anyhow::{anyhow, Context, Result};
use rbatis::RBatis;
use std::collections::BTreeSet;
use std::path::Path;
use tracing::{error, info};

/// 执行启动期数据库迁移
pub async fn run(rb: &RBatis, db_path: &Path) -> Result<()> {
    rb.exec(migrate::CREATE_META_SQL, vec![])
        .await
        .context("创建 app_meta 表失败")?;

    let state = ledger::inspect(rb).await?;

    // 收敛要 DROP 表，快照不能省；`Missing` / `Ready` 不动已有数据，不备份
    if state.needs_rebuild() {
        snapshot::database(rb, db_path).await;
    }
    match state {
        ledger::State::Missing => ledger::create(rb).await?,
        ledger::State::Ready => {}
        ledger::State::Rebuild { kind, rows } => ledger::rebuild(rb, kind, rows).await?,
    }

    let applied: BTreeSet<i64> = migrate::select_applied(rb)
        .await
        .context("读取迁移账本失败")?
        .into_iter()
        .map(|row| row.version)
        .collect();

    let pending: Vec<&sql::SqlMigration> = sql::MIGRATIONS
        .iter()
        .filter(|migration| !applied.contains(&migration.version))
        .collect();

    if pending.is_empty() {
        info!("数据库结构已是最新（V{}）", latest_version());
        return Ok(());
    }

    // 只有"已经在用的库"才需要备份；全新安装备份一个空库没有意义
    if !state.needs_rebuild() && !applied.is_empty() {
        snapshot::database(rb, db_path).await;
    }

    let planned: Vec<String> = pending.iter().map(|m| format!("V{}", m.version)).collect();
    info!("待执行迁移 {} 条: {}", pending.len(), planned.join(", "));

    apply_batch(rb, &pending).await?;
    info!("迁移完成，数据库结构已更新到 V{}", latest_version());
    Ok(())
}

fn latest_version() -> i64 {
    sql::MIGRATIONS.last().map(|m| m.version).unwrap_or(0)
}

async fn apply_batch(rb: &RBatis, pending: &[&sql::SqlMigration]) -> Result<()> {
    let conn = rb.try_acquire().await.context("获取迁移连接失败")?;
    conn.exec("PRAGMA foreign_keys = OFF", vec![])
        .await
        .context("关闭外键约束失败")?;
    let mut tx = conn
        .begin()
        .await
        .context("开启迁移事务失败，该连接的外键约束已关闭，进程随后退出")?;

    let outcome = async {
        for migration in pending {
            let statements = sql::normalize(migration.sql);
            tx.exec(&statements, vec![])
                .await
                .map_err(|e| anyhow!("V{}（{}）执行失败: {e}", migration.version, migration.name))?;
        }

        // 外键强制已关闭，中间态与最终态依赖这一步（`foreign_key_check` 独立于 enforcement）
        let violations = migrate::foreign_key_violations(&mut tx).await.context("外键检查失败")?;
        if violations > 0 {
            return Err(anyhow!("迁移后存在 {violations} 条外键违例，整批回滚"));
        }

        // 账本时间戳与业务表同格式：规范 ISO 8601 串，不存毫秒整数
        let migrated_at = current_iso_8601();
        for migration in pending {
            migrate::insert_applied(&mut tx, migration.version, &migrated_at)
                .await
                .with_context(|| format!("记录 V{} 到迁移账本失败", migration.version))?;
        }
        Ok::<(), anyhow::Error>(())
    }
        .await;

    match outcome {
        Ok(()) => {
            tx.commit().await.context("提交迁移事务失败")?;
            restore_foreign_keys(&tx).await;
            for migration in pending {
                info!("✅ 迁移完成: V{} {}", migration.version, migration.name);
            }
            Ok(())
        }
        Err(e) => {
            if let Err(rollback) = tx.rollback().await {
                error!("回滚迁移事务失败: {rollback}");
            }
            restore_foreign_keys(&tx).await;
            Err(e)
        }
    }
}

/// 恢复本条连接的外键强制
async fn restore_foreign_keys(tx: &rbatis::RBatisTxExecutor) {
    if let Err(e) = tx.exec("PRAGMA foreign_keys = ON", vec![]).await {
        error!("为连接 {} 恢复外键约束失败，具体错误为：{e}。该连接将失去外键保护！", tx.tx_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::time_utils::current_timestamp_millis;
    use rbdc_sqlite::SqliteDriver;
    use std::path::PathBuf;

    fn temp_db() -> PathBuf {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "rollcaller-migrate-{}-{}-{}.db",
            std::process::id(),
            current_timestamp_millis(),
            seq
        ))
    }

    #[test]
    fn normalize_is_idempotent() {
        let once = sql::normalize(sql::MIGRATIONS[0].sql);
        assert_eq!(once, sql::normalize(&once));
        assert!(!once.contains('\r'));
    }

    /// 外键约束由驱动默认开启
    #[test]
    fn foreign_keys_are_enabled() {
        tauri::async_runtime::block_on(async {
            let path = temp_db();
            let rb = RBatis::new();
            rb.link(SqliteDriver {}, &format!("sqlite:{}", path.display())).await.unwrap();

            let enabled: i64 = rb.exec_decode("PRAGMA foreign_keys", vec![]).await.unwrap();
            assert_eq!(enabled, 1);

            drop(rb);
            let _ = std::fs::remove_file(&path);
        });
    }

    /// 空库上跑完全部迁移：建表 + 账本记账，重复执行不再动
    #[test]
    fn fresh_database_runs_all_migrations_once() {
        tauri::async_runtime::block_on(async {
            let path = temp_db();
            let rb = RBatis::new();
            rb.link(SqliteDriver {}, &format!("sqlite:{}", path.display())).await.unwrap();

            run(&rb, &path).await.unwrap();

            let tables: i64 = rb
                .exec_decode(
                    "SELECT COUNT(*) FROM sqlite_master
                     WHERE type = 'table'
                       AND name IN ('students', 'records', 'attendance_status_definition',
                                    'database_migration_history', 'app_meta')",
                    vec![],
                )
                .await
                .unwrap();
            assert_eq!(tables, 5, "业务表与账本表都应已创建");

            let versions: Vec<i64> = migrate::select_applied(&rb)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.version)
                .collect();
            assert_eq!(versions, vec![0, 20, 30, 40]);

            // 再跑一次，全部跳过，不应重复执行 V30 的 +1
            run(&rb, &path).await.unwrap();
            let total: i64 = rb
                .exec_decode("SELECT COUNT(*) FROM database_migration_history", vec![])
                .await
                .unwrap();
            assert_eq!(total, 4);

            // 全新库首次迁移不该留下备份文件
            assert_eq!(backup_count(&path), 0, "全新库迁移不应产生快照");

            drop(rb);
            let _ = std::fs::remove_file(&path);
        });
    }

    /// V40：时间列必须变成定长的 ISO 8601 UTC 串，且与原毫秒值等价
    #[test]
    fn v40_converts_timestamps_to_iso8601() {
        tauri::async_runtime::block_on(async {
            let path = temp_db();
            let rb = RBatis::new();
            rb.link(SqliteDriver {}, &format!("sqlite:{}", path.display())).await.unwrap();

            // 手工建到 V30：执行前三条迁移并记账，让 V40 成为唯一待执行项
            rb.exec(migrate::CREATE_HISTORY_SQL, vec![]).await.unwrap();
            for migration in &sql::MIGRATIONS[..3] {
                rb.exec(&sql::normalize(migration.sql), vec![]).await.unwrap();
                rb.exec(
                    &format!(
                        "INSERT INTO database_migration_history (version, migrated_at) \
                         VALUES ({}, '2026-01-01T00:00:00.000Z')",
                        migration.version
                    ),
                    vec![],
                )
                    .await
                    .unwrap();
            }

            // V30 之后：状态码已 +1，records 带两个外键
            rb.exec(
                "INSERT INTO students (student_no, name, created_at, updated_at) VALUES
                     ('001', '张三', 1786421798789, 1786421786659);
                 INSERT INTO students (student_no, name, created_at, updated_at, is_deleted, deleted_at) VALUES
                     ('002', '李四', 1700000000000, 1700000000001, 1, 1700000000002);
                 INSERT INTO records (student_id, attendance_status, rollcall_at, session_id) VALUES
                     (1, 2, 1786421798789, 'sess-1');",
                vec![],
            )
                .await
                .unwrap();

            run(&rb, &path).await.unwrap();

            let created: String = rb
                .exec_decode("SELECT created_at FROM students WHERE student_no = '001'", vec![])
                .await
                .unwrap();
            assert_eq!(created, "2026-08-11T04:16:38.789Z");

            let rollcall: String = rb
                .exec_decode("SELECT rollcall_at FROM records WHERE session_id = 'sess-1'", vec![])
                .await
                .unwrap();
            assert_eq!(rollcall, "2026-08-11T04:16:38.789Z");

            // NULL 必须原样保留，不能被转成 "0" 或空串
            let deleted: Option<String> = rb
                .exec_decode("SELECT deleted_at FROM records WHERE session_id = 'sess-1'", vec![])
                .await
                .unwrap();
            assert_eq!(deleted, None);

            // 软删除时间也转过来了
            let soft_deleted: String = rb
                .exec_decode("SELECT deleted_at FROM students WHERE student_no = '002'", vec![])
                .await
                .unwrap();
            assert_eq!(soft_deleted, "2023-11-14T22:13:20.002Z");

            // 重建后的触发器继续产出规范串
            rb.exec("UPDATE students SET name = '张三三' WHERE student_no = '001'", vec![])
                .await
                .unwrap();
            let updated: String = rb
                .exec_decode("SELECT updated_at FROM students WHERE student_no = '001'", vec![])
                .await
                .unwrap();
            assert_eq!(updated.len(), 24, "触发器应产出定长 ISO 串，实得 {updated}");
            assert!(updated.ends_with('Z'));

            // 外键强制已恢复，且当前库无违例
            let enabled: i64 = rb.exec_decode("PRAGMA foreign_keys", vec![]).await.unwrap();
            assert_eq!(enabled, 1, "迁移后必须恢复外键强制");
            let violations: i64 = rb
                .exec_decode("SELECT COUNT(*) FROM pragma_foreign_key_check", vec![])
                .await
                .unwrap();
            assert_eq!(violations, 0);

            drop(rb);
            let _ = std::fs::remove_file(&path);
        });
    }

    fn backup_count(path: &Path) -> usize {
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(&format!("{stem}-bak-")))
            .count()
    }

    fn cleanup(path: &Path) {
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        if let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with(&stem) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        let _ = std::fs::remove_file(path);
    }

    /// 旧账本收敛 + 无待执行迁移：只应留下一份快照，不重复备份
    #[test]
    fn legacy_rebuild_takes_exactly_one_snapshot() {
        tauri::async_runtime::block_on(async {
            let path = temp_db();
            let rb = RBatis::new();
            rb.link(SqliteDriver {}, &format!("sqlite:{}", path.display())).await.unwrap();
            rb.exec(
                "CREATE TABLE migration_history (
                     version INTEGER PRIMARY KEY, executed_at INTEGER, created_at INTEGER
                 );",
                vec![],
            )
            .await
            .unwrap();
            // 账本声称全部迁移已应用（含最新的 V40），但本库没建任何业务表：
            // 因此待执行集合为空，只应发生账本收敛
            rb.exec(
                "INSERT INTO migration_history (version, executed_at) VALUES (0, 1), (20, 2), (30, 3), (40, 4);",
                vec![],
            )
            .await
            .unwrap();

            run(&rb, &path).await.unwrap();

            assert_eq!(backup_count(&path), 1, "旧账本收敛只该打一份快照");

            let total: i64 = rb
                .exec_decode("SELECT COUNT(*) FROM database_migration_history", vec![])
                .await
                .unwrap();
            assert_eq!(total, 4);

            drop(rb);
            cleanup(&path);
        });
    }

    /// 最真实的一条升级路径：V30 的库（账本还是 INTEGER 毫秒）+ 待执行 V40
    ///
    /// 同一次启动里先改写账本、再跑 V40，两者都成功且只打一份快照；
    /// 第二次启动必须什么都不做。
    #[test]
    fn upgrade_from_v30_rewrites_ledger_then_applies_v40() {
        tauri::async_runtime::block_on(async {
            let path = temp_db();
            let rb = RBatis::new();
            rb.link(SqliteDriver {}, &format!("sqlite:{}", path.display())).await.unwrap();

            // 旧形状账本：`migrated_at` 声明为 INTEGER，值是毫秒
            rb.exec(
                "CREATE TABLE database_migration_history (
                     version INTEGER PRIMARY KEY, migrated_at INTEGER NOT NULL
                 );",
                vec![],
            )
                .await
                .unwrap();
            for migration in &sql::MIGRATIONS[..3] {
                rb.exec(&sql::normalize(migration.sql), vec![]).await.unwrap();
            }
            rb.exec(
                "INSERT INTO database_migration_history (version, migrated_at)
                 VALUES (0, 1786421798789), (20, 1786421786659), (30, 1786421000000);
                 INSERT INTO students (student_no, name, created_at, updated_at)
                 VALUES ('001', '张三', 1786421798789, 1786421786659);
                 INSERT INTO records (student_id, attendance_status, rollcall_at, session_id)
                 VALUES (1, 2, 1786421798789, 'sess-1');",
                vec![],
            )
                .await
                .unwrap();

            run(&rb, &path).await.unwrap();

            // 账本：列类型 TEXT、值 ISO、旧版本一个不少，V40 也记上了
            let column_type =
                migrate::column_type(&rb, migrate::HISTORY_TABLE, "migrated_at").await.unwrap();
            assert!(column_type.eq_ignore_ascii_case("TEXT"), "账本列应改为 TEXT: {column_type}");
            let versions: Vec<i64> = migrate::select_applied(&rb)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.version)
                .collect();
            assert_eq!(versions, vec![0, 20, 30, 40]);
            let stamp: String = rb
                .exec_decode(
                    "SELECT migrated_at FROM database_migration_history WHERE version = 0",
                    vec![],
                )
                .await
                .unwrap();
            assert_eq!(stamp, "2026-08-11T04:16:38.789Z", "旧毫秒值必须等值换算");

            // 业务表：V40 已生效
            let created: String = rb
                .exec_decode("SELECT created_at FROM students WHERE student_no = '001'", vec![])
                .await
                .unwrap();
            assert_eq!(created, "2026-08-11T04:16:38.789Z");
            let rollcall: String = rb
                .exec_decode("SELECT rollcall_at FROM records WHERE session_id = 'sess-1'", vec![])
                .await
                .unwrap();
            assert_eq!(rollcall, "2026-08-11T04:16:38.789Z");

            // 账本改写与 V40 在同一次启动里完成，只该打一份快照
            assert_eq!(backup_count(&path), 1, "一次升级只该打一份快照");

            // 再启动：无待执行迁移、账本已就绪，不应再备份
            run(&rb, &path).await.unwrap();
            assert_eq!(backup_count(&path), 1, "重复启动不应产生新快照");

            drop(rb);
            cleanup(&path);
        });
    }
}
