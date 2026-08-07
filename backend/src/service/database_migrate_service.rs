use crate::config::app_paths;
use crate::database::database_pool;
use crate::repo::database_migration_repo;
use crate::repo::database_migration_repo::update_success_status;
use crate::util::time_utils::current_timestamp_millis;
use anyhow::{anyhow, Context};
use jiff::Timestamp;
use rbatis::{RBatis, RBatisRef, RBatisTxExecutor};
use rbs::{value, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use crate::common::ext::hash_ext::HashExt;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};

pub struct MigrationFileInfo {
    pub path: PathBuf,
    pub version: i64,
    pub description: String,
    pub content: String,
    pub checksum: String,
}

pub async fn migrate(rb: &RBatis, migration_dir: &Path) -> anyhow::Result<()> {
    //* 1. 创建迁移历史表
    database_migration_repo::create_migration_history_table(rb)
        .await
        .map_err(|e| anyhow!("创建迁移历史表 migration_history 失败: {}", e))?;

    //* 2. 读取并解析迁移文件
    let migration_files = load_files(migration_dir)
        .with_context(|| format!("读取迁移文件失败, 迁移目录: {}", migration_dir.display()))?;

    //* 3. 获取已执行的迁移记录 (MigrationRecord {version, checksum, status})
    let history = database_migration_repo::get_existing_history(rb)
        .await
        .map_err(|e| anyhow!("读取已执行的迁移记录失败: {}", e))?;

    //* 4. 开启事务
    let mut tx = rb.try_acquire_begin().await?;

    //* 5. 执行迁移
    for file in &migration_files {
        // 获取已执行的迁移
        let record = history.iter().find(|r| r.version == file.version);

        match record {
            //* 5.1 迁移记录存在 且 校验和相同
            Some(expected) if expected.checksum == file.checksum => {
                info!(
                    "⏭️  跳过已执行的迁移: V{}__{}.sql",
                    file.version, file.description
                );
                // 此处无需提交事务，需接着遍历，执行迁移
                if expected.status == "FAILED" {
                    update_success_status(&mut tx, expected.version, current_timestamp_millis())
                        .await?;
                }
            }
            //* 5.2 迁移记录存在 但 校验和不同
            Some(expected) => {
                let err_msg = format!(
                    "Checksum 不匹配: V{}__{}.sql 期望: [{}], 实际: [{}]",
                    file.version, file.description, expected.checksum, file.checksum
                );
                error!(err_msg);
                //* 6. 回滚事务
                tx.rollback().await?;
                //. 此记录是实际存在的，且已经执行过迁移
                //. 因此需要在新事务中更新状态为 FAILED
                let mut new_tx = rb.try_acquire_begin().await?;
                database_migration_repo::update_failed_status(&mut new_tx, file.version, &err_msg)
                    .await?;
                new_tx.commit().await?;
                info!("事务已回滚：{}", tx.tx_id);
                return Err(anyhow!(err_msg));
            }
            //* 5.3 迁移记录不存在
            None => {
                // 未执行迁移
                info!("📄 执行迁移: V{}__{}.sql", file.version, file.description);
                // 先插入新纪录，status默认为 'RUNNING_IN_TX'
                database_migration_repo::insert_new_history(
                    &mut tx,
                    file.version,
                    &file.description,
                    &file.content,
                    &file.checksum,
                )
                .await?;

                // 执行迁移 SQL
                if let Err(e) = tx.exec(&file.content, vec![]).await {
                    let err_msg = format!(
                        "执行失败: V{}__{}.sql - {}",
                        file.version, file.description, e
                    );
                    //* 6. 回滚事务
                    //. 由于已经回滚，因此该INSERT记录在磁盘中实际不存在，无需设置为FAILED
                    tx.rollback().await?;
                    error!(err_msg);
                    return Err(anyhow!(err_msg));
                }

                // 如果成功，更新为 SUCCESS
                update_success_status(&mut tx, file.version, current_timestamp_millis()).await?;
                info!("✅ 迁移成功: V{}__{}.sql", file.version, file.description);
            }
        }
    }

    //* 7. 提交事务
    tx.commit().await?;
    info!("🎉 所有迁移执行完成");
    Ok(())
}

fn load_files(migration_dir: &Path) -> anyhow::Result<Vec<MigrationFileInfo>> {
    let mut migration_files = Vec::new();
    let entries = fs::read_dir(migration_dir)
        .with_context(|| format!("无法打开迁移目录: {}", migration_dir.display()))?;
    for entry in entries {
        let entry = entry
            .with_context(|| format!("读取迁移目录条目失败: {}", migration_dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("sql") {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if let Some((version, description)) = parse_filename(file_name) {
            let content = fs::read_to_string(&path)?;
            let checksum = content.sha256();
            migration_files.push(MigrationFileInfo {
                path,
                version,
                description,
                content,
                checksum,
            });
        }
    }
    migration_files.sort_by_key(|m| m.version);
    Ok(migration_files)
}

/// 解析文件名: V1__init.sql -> (1, "init")
fn parse_filename(filename: &str) -> Option<(i64, String)> {
    let name = filename.strip_suffix(".sql")?;
    let parts: Vec<&str> = name.split("__").collect();
    if parts.len() != 2 || !parts[0].starts_with('V') {
        return None;
    }
    let version = parts[0].trim_start_matches('V').parse::<i64>().ok()?;
    let description = parts[1].replace("-", " ");
    debug!("解析迁移文件名：[{}] -> [{}] + [{}]", filename, &version, &description);
    Some((version, description))
}
