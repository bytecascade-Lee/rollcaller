//! 迁移账本的形状收敛
//!
//! 账本表无用"迁移"来修改自己。所以它由启动时的**幂等收敛**负责，通过列集决定"新建 / 已就绪 / 从旧表重建"

use crate::database::migration::migrate;
use crate::util::time_utils::{current_iso_8601, current_timestamp_millis};
use anyhow::{anyhow, Context, Result};
use rbatis::{RBatis, RBatisTxExecutor};
use tracing::{info, warn};

/// 账本当前所处的形状
#[derive(Debug, Clone, Copy)]
pub(crate) enum State {
    /// 新旧表都不存在：全新数据库
    Missing,
    /// 已经是 `version` + `migrated_at`
    Ready,
    /// 旧账本表待收敛；`rows` 是旧表行数，用于日志与搬迁后的行数校验
    Legacy { rows: i64 },
}

/// 探测账本形状
pub(crate) async fn inspect(rb: &RBatis) -> Result<State> {
    // *1. 新表在：确认关键列齐全，多余列容忍
    if migrate::table_exists(rb, migrate::HISTORY_TABLE).await? > 0 {
        let ready = migrate::column_exists(rb, migrate::HISTORY_TABLE, "version").await? > 0
            && migrate::column_exists(rb, migrate::HISTORY_TABLE, "migrated_at").await? > 0;
        if !ready {
            return Err(anyhow!(
                "账本表 {} 缺少 version / migrated_at 列，拒绝启动",
                migrate::HISTORY_TABLE
            ));
        }
        return Ok(State::Ready);
    }

    // *2. 新表不在、旧表也不在：全新数据库
    if migrate::table_exists(rb, migrate::LEGACY_TABLE).await? == 0 {
        return Ok(State::Missing);
    }

    // *3. 只剩旧表：必须能认出它的版本列，否则交给人处理
    if migrate::column_exists(rb, migrate::LEGACY_TABLE, "version").await? == 0 {
        return Err(anyhow!(
            "旧账本表 {} 缺少 version 列，无法自动收敛；请备份数据库后手动处理",
            migrate::LEGACY_TABLE
        ));
    }
    let rows = migrate::count_legacy_rows(rb).await.context("读取旧账本行数失败")?;
    Ok(State::Legacy { rows })
}

/// 全新库：建账本表
pub(crate) async fn create(rb: &RBatis) -> Result<()> {
    rb.exec(migrate::CREATE_HISTORY_SQL, vec![]).await.context("创建账本表失败")?;
    info!("已创建迁移账本表 {}", migrate::HISTORY_TABLE);
    Ok(())
}

/// 旧账本 -> 新账本：整表搬迁后删旧表，行数对不上就整批回滚
pub(crate) async fn rebuild(rb: &RBatis, before: i64) -> Result<()> {
    info!(
        "检测到旧账本表 {}，共 {before} 行，开始收敛到 {}",
        migrate::LEGACY_TABLE,
        migrate::HISTORY_TABLE
    );

    let mut tx = rb.try_acquire_begin().await.context("开启账本收敛事务失败")?;
    match rebuild_in_tx(&mut tx, before).await {
        Ok(()) => {
            tx.commit().await.context("提交账本收敛事务失败")?;
            if let Err(e) = migrate::upsert_meta(rb, "ledger_rebuilt_at", &current_iso_8601()).await {
                warn!("写入 app_meta.ledger_rebuilt_at 失败: {e}");
            }
            info!("账本收敛完成：{before} 条记录已保留");
            Ok(())
        }
        Err(e) => {
            if let Err(rollback) = tx.rollback().await {
                warn!("回滚账本收敛事务失败: {rollback}");
            }
            Err(e)
        }
    }
}

async fn rebuild_in_tx(tx: &mut RBatisTxExecutor, before: i64) -> Result<()> {
    tx.exec(migrate::CREATE_HISTORY_SQL, vec![])
        .await
        .context("创建新账本表失败")?;

    let copied = migrate::copy_legacy_rows(tx).await.context("搬迁旧账本记录失败")?;
    let after = copied.rows_affected as i64;
    if after != before {
        return Err(anyhow!("账本收敛行数不一致（{before} -> {after}），已放弃"));
    }

    tx.exec(&format!("DROP TABLE {}", migrate::LEGACY_TABLE), vec![])
        .await
        .context("删除旧账本表失败")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbdc_sqlite::SqliteDriver;
    use std::path::PathBuf;

    /// 重构前的账本表
    const LEGACY_DDL: &str = "
        CREATE TABLE migration_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            version INTEGER NOT NULL UNIQUE,
            description TEXT NOT NULL,
            content TEXT NOT NULL,
            checksum TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'RUNNING_IN_TX',
            executed_at INTEGER,
            created_at INTEGER DEFAULT 0,
            updated_at INTEGER DEFAULT 0,
            remark TEXT
        );
        CREATE TRIGGER trg_migration_history_after_update
            AFTER UPDATE ON migration_history
        BEGIN
            UPDATE migration_history SET updated_at = 1 WHERE id = NEW.id;
        END;
    ";

    const LEGACY_ROWS: &str = "
        INSERT INTO migration_history (version, description, content, checksum, status, executed_at)
        VALUES (0,  'initialize schema',                  'CREATE TABLE students();', 'aaa', 'SUCCESS', 111),
               (10, 'update v1 description',              'UPDATE migration_history', 'bbb', 'FAILED',  222),
               (20, 'create attendance_status_definition', 'CREATE TABLE asd();',     'ccc', 'SUCCESS', 333);
    ";

    fn temp_db() -> PathBuf {
        // 测试并行执行：毫秒 + 自增序号，避免两个用例撞到同一个文件名
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "rollcaller-ledger-{}-{}-{}.db",
            std::process::id(),
            current_timestamp_millis(),
            seq
        ))
    }

    /// 收敛旧表；调用方负责先打快照
    async fn converge(rb: &RBatis) -> Result<()> {
        match inspect(rb).await? {
            State::Missing => create(rb).await,
            State::Ready => Ok(()),
            State::Legacy { rows } => rebuild(rb, rows).await,
        }
    }

    #[test]
    fn legacy_ledger_is_rebuilt_and_preserved() {
        tauri::async_runtime::block_on(async {
            let path = temp_db();
            let rb = RBatis::new();
            rb.link(SqliteDriver {}, &format!("sqlite:{}", path.display())).await.unwrap();
            rb.exec(LEGACY_DDL, vec![]).await.unwrap();
            rb.exec(LEGACY_ROWS, vec![]).await.unwrap();

            assert!(matches!(inspect(&rb).await.unwrap(), State::Legacy { rows: 3 }));
            converge(&rb).await.unwrap();

            let versions: Vec<i64> = migrate::select_applied(&rb)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.version)
                .collect();
            assert_eq!(versions, vec![0, 10, 20], "已应用集合必须原样保留");
            assert_eq!(
                migrate::table_exists(&rb, migrate::LEGACY_TABLE).await.unwrap(),
                0,
                "旧表应已删除"
            );
            assert_eq!(migrate::table_exists(&rb, migrate::HISTORY_TABLE).await.unwrap(), 1);

            // FAILED 行代表"曾成功应用过"，时间戳也要留住
            let migrated_at: i64 = rb
                .exec_decode(
                    "SELECT migrated_at FROM database_migration_history WHERE version = 10",
                    vec![],
                )
                .await
                .unwrap();
            assert_eq!(migrated_at, 222);

            // 幂等：再跑一次不报错、不重复
            assert!(matches!(inspect(&rb).await.unwrap(), State::Ready));
            converge(&rb).await.unwrap();
            let total: i64 = rb
                .exec_decode("SELECT COUNT(*) FROM database_migration_history", vec![])
                .await
                .unwrap();
            assert_eq!(total, 3);

            drop(rb);
            let _ = std::fs::remove_file(&path);
        });
    }

    #[test]
    fn fresh_database_gets_an_empty_ledger() {
        tauri::async_runtime::block_on(async {
            let path = temp_db();
            let rb = RBatis::new();
            rb.link(SqliteDriver {}, &format!("sqlite:{}", path.display())).await.unwrap();

            assert!(matches!(inspect(&rb).await.unwrap(), State::Missing));
            converge(&rb).await.unwrap();

            assert_eq!(migrate::table_exists(&rb, migrate::HISTORY_TABLE).await.unwrap(), 1);
            assert!(migrate::select_applied(&rb).await.unwrap().is_empty());

            drop(rb);
            let _ = std::fs::remove_file(&path);
        });
    }

    /// 账本表存在但缺关键列：拒绝启动，不许建空表顶上去
    #[test]
    fn unrecognized_ledger_is_refused() {
        tauri::async_runtime::block_on(async {
            let path = temp_db();
            let rb = RBatis::new();
            rb.link(SqliteDriver {}, &format!("sqlite:{}", path.display())).await.unwrap();
            rb.exec("CREATE TABLE database_migration_history (id INTEGER PRIMARY KEY)", vec![])
                .await
                .unwrap();

            let error = inspect(&rb).await.unwrap_err();
            assert!(error.to_string().contains("拒绝启动"), "{error}");

            drop(rb);
            let _ = std::fs::remove_file(&path);
        });
    }
}
