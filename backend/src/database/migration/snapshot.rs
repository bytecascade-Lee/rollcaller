//! 迁移前给数据库留一份快照
//!
//! 用 `VACUUM INTO` 生成独立副本，连同 WAL 里尚未回写的页，备份失败只告警

use crate::util::time_utils::current_timestamp_millis;
use rbatis::RBatis;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// 同类快照最多保留几份
const KEEP: usize = 3;

/// 打一份快照，并清理过旧的同类文件
pub(crate) async fn database(rb: &RBatis, source: &Path) {
    let target = unique_target(source);
    let literal = target.display().to_string().replace('\'', "''");
    match rb.exec(&format!("VACUUM INTO '{literal}'"), vec![]).await {
        Ok(_) => {
            info!("已备份数据库: {}", target.display());
            prune(source);
        }
        Err(e) => warn!("数据库备份失败（继续迁移，迁移本身在事务内）: {e}"),
    }
}

fn unique_target(source: &Path) -> PathBuf {
    let file_name = format!("{}-bak-{}.db", stem(source), &uuid::Uuid::new_v4().to_string()[..8]);
    source.with_file_name(file_name)
}

fn stem(source: &Path) -> &str {
    source.file_stem().and_then(|name| name.to_str()).unwrap_or("sqlite")
}

/// 只清理本模块生成的 `<stem>-bak-*.db`，保留最近 [`KEEP`] 份
fn prune(source: &Path) {
    let prefix = format!("{}-bak-", stem(source));
    let Some(parent) = source.parent() else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };

    let mut backups: Vec<(std::time::SystemTime, PathBuf)> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with(&prefix) && name.ends_with(".db")
        })
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .collect();

    if backups.len() <= KEEP {
        return;
    }
    backups.sort_by(|a, b| b.0.cmp(&a.0)); // 新的在前
    for (_, path) in backups.into_iter().skip(KEEP) {
        match std::fs::remove_file(&path) {
            Ok(()) => info!("已清理过旧备份: {}", path.display()),
            Err(e) => warn!("清理过旧备份失败 {}: {e}", path.display()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbdc_sqlite::SqliteDriver;

    fn temp_db() -> PathBuf {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "rollcaller-snapshot-{}-{}-{}.db",
            std::process::id(),
            current_timestamp_millis(),
            seq
        ))
    }

    fn count_backups(source: &Path) -> usize {
        let prefix = format!("{}-bak-", stem(source));
        std::fs::read_dir(source.parent().unwrap())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
            .count()
    }

    /// 快照可重复生成，且只保留最近 `KEEP` 份
    #[test]
    fn snapshot_keeps_only_recent_backups() {
        tauri::async_runtime::block_on(async {
            let path = temp_db();
            let rb = RBatis::new();
            rb.link(SqliteDriver {}, &format!("sqlite:{}", path.display())).await.unwrap();

            for _ in 0..(KEEP + 2) {
                database(&rb, &path).await;
            }
            assert_eq!(count_backups(&path), KEEP, "应只保留最近 {KEEP} 份快照");

            for entry in std::fs::read_dir(path.parent().unwrap()).unwrap().flatten() {
                if entry.file_name().to_string_lossy().starts_with(stem(&path)) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
            drop(rb);
            let _ = std::fs::remove_file(&path);
        });
    }
}
