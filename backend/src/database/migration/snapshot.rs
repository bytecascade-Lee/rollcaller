//! 迁移前给数据库留一份快照
//!
//! 用 `VACUUM INTO` 生成独立副本，连同 WAL 里尚未回写的页；备份失败只告警，不阻断迁移

use rbatis::RBatis;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// 发布版同类快照的保留份数
const KEEP: usize = 3;

const KEEP_LIMIT: Option<usize> = if cfg!(debug_assertions) { None } else { Some(KEEP) };

/// 打一份快照，并按当前构建的保留策略清理过旧备份
pub(crate) async fn database(rb: &RBatis, source: &Path) {
    let target = unique_target(source);
    let literal = target.display().to_string().replace('\'', "''");
    match rb.exec(&format!("VACUUM INTO '{literal}'"), vec![]).await {
        Ok(_) => {
            info!("已备份数据库: {}", target.display());
            prune(source, KEEP_LIMIT);
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

/// 只清理本模块生成的 `<stem>-bak-*.db`，保留最近的 `keep` 份
fn prune(source: &Path, keep: Option<usize>) {
    let Some(keep) = keep else {
        return;
    };
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

    if backups.len() <= keep {
        return;
    }
    backups.sort_by(|a, b| b.0.cmp(&a.0)); // 新的在前
    for (_, path) in backups.into_iter().skip(keep) {
        match std::fs::remove_file(&path) {
            Ok(()) => info!("已清理过旧备份: {}", path.display()),
            Err(e) => warn!("清理过旧备份失败 {}: {e}", path.display()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::time_utils::current_timestamp_millis;
    use rbdc_sqlite::SqliteDriver;
    use std::time::{Duration, SystemTime};

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

    fn backup_names(source: &Path) -> Vec<String> {
        let prefix = format!("{}-bak-", stem(source));
        let mut names: Vec<String> = std::fs::read_dir(source.parent().unwrap())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(&prefix))
            .collect();
        names.sort();
        names
    }

    /// 造 `n` 份假快照，修改时间依次递增一小时（第 `i` 份最旧）
    fn touch_backups(source: &Path, n: usize) -> Vec<String> {
        let base = SystemTime::now() - Duration::from_secs(86_400);
        let mut names = Vec::with_capacity(n);
        for i in 0..n {
            let file_name = format!("{}-bak-{:08x}.db", stem(source), i);
            let path = source.with_file_name(&file_name);
            std::fs::write(&path, b"x").unwrap();
            std::fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(base + Duration::from_secs(3_600 * (i as u64 + 1)))
                .unwrap();
            names.push(file_name);
        }
        names
    }

    fn cleanup(source: &Path) {
        if let Some(parent) = source.parent() {
            if let Ok(entries) = std::fs::read_dir(parent) {
                for entry in entries.flatten() {
                    if entry.file_name().to_string_lossy().starts_with(stem(source)) {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }
        let _ = std::fs::remove_file(source);
    }

    /// 发布版策略：超出上限时删最旧的，留下的必须是最近 `KEEP` 份
    #[test]
    fn prune_keeps_the_newest_backups() {
        let source = temp_db();
        let names = touch_backups(&source, KEEP + 2);

        prune(&source, Some(KEEP));

        assert_eq!(count_backups(&source), KEEP, "应只保留最近 {KEEP} 份");
        let kept = backup_names(&source);
        let newest: Vec<String> = names[names.len() - KEEP..].to_vec();
        assert_eq!(kept, newest, "被删的必须是最旧的两份");

        cleanup(&source);
    }

    /// 未超上限时不做任何删除
    #[test]
    fn prune_leaves_short_history_alone() {
        let source = temp_db();
        let names = touch_backups(&source, KEEP);

        prune(&source, Some(KEEP));

        assert_eq!(backup_names(&source), names);
        cleanup(&source);
    }

    /// 开发模式策略：`None` 表示一份都不删
    #[test]
    fn prune_without_limit_keeps_everything() {
        let source = temp_db();
        let names = touch_backups(&source, KEEP + 3);

        prune(&source, None);

        assert_eq!(backup_names(&source), names, "开发模式不应删除任何备份");
        cleanup(&source);
    }

    /// 真实快照走 [`database`]，清理结果必须与本次构建的保留策略一致
    #[test]
    fn snapshot_follows_retention_policy() {
        tauri::async_runtime::block_on(async {
            let path = temp_db();
            let rb = RBatis::new();
            rb.link(SqliteDriver {}, &format!("sqlite:{}", path.display())).await.unwrap();

            let rounds = KEEP + 2;
            for _ in 0..rounds {
                database(&rb, &path).await;
            }
            let expected = KEEP_LIMIT.map_or(rounds, |keep| keep.min(rounds));
            assert_eq!(count_backups(&path), expected, "保留份数与 KEEP_LIMIT 不符");

            drop(rb);
            cleanup(&path);
        });
    }
}
