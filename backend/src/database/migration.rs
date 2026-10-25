//! 数据库结构迁移

mod ledger;
mod migrate;
mod sql;

use crate::util::time_utils::current_timestamp_millis;
use anyhow::{anyhow, Context, Result};
use rbatis::RBatis;
use std::collections::BTreeSet;
use std::path::Path;
use tracing::{error, info};

/// 执行启动期数据库迁移
pub async fn run(rb: &RBatis, db_path: &Path) -> Result<()> {
    ledger::ensure_schema(rb, db_path).await?;

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
    let mut tx = rb.try_acquire_begin().await.context("开启迁移事务失败")?;

    let outcome = async {
        // 重建表类迁移的中间态会短暂违反外键，延迟到提交时统一校验
        tx.exec("PRAGMA defer_foreign_keys = ON", vec![])
            .await
            .context("设置延迟外键校验失败")?;

        for migration in pending {
            let statements = sql::normalize(migration.sql);
            tx.exec(&statements, vec![])
                .await
                .map_err(|e| anyhow!("V{}（{}）执行失败: {e}", migration.version, migration.name))?;
        }

        let violations = migrate::foreign_key_violations(&mut tx).await.context("外键检查失败")?;
        if violations > 0 {
            return Err(anyhow!("迁移后存在 {violations} 条外键违例，整批回滚"));
        }

        let applied_at = current_timestamp_millis();
        for migration in pending {
            migrate::insert_applied(&mut tx, migration.version, applied_at)
                .await
                .with_context(|| format!("记录 V{} 到迁移账本失败", migration.version))?;
        }
        Ok::<(), anyhow::Error>(())
    }
        .await;

    match outcome {
        Ok(()) => {
            tx.commit().await.context("提交迁移事务失败")?;
            for migration in pending {
                info!("✅ 迁移完成: V{} {}", migration.version, migration.name);
            }
            Ok(())
        }
        Err(e) => {
            if let Err(rollback) = tx.rollback().await {
                error!("回滚迁移事务失败: {rollback}");
            }
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            assert_eq!(versions, vec![0, 20, 30]);

            // 再跑一次，全部跳过，不应重复执行 V30 的 +1
            run(&rb, &path).await.unwrap();
            let total: i64 = rb
                .exec_decode("SELECT COUNT(*) FROM database_migration_history", vec![])
                .await
                .unwrap();
            assert_eq!(total, 3);

            drop(rb);
            let _ = std::fs::remove_file(&path);
        });
    }
}
