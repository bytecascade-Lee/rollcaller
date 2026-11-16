//! 日志文件回收
//!
//! - 0 字节空文件
//! - 超出时间范围
//! - 超出空间占用

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// 超量时的清理目标，上限的四分之三
///
/// 不删到刚好等于上限，否则下次启动立刻又超限，会变成"每次启动均删除"的抖动
const TARGET_RATIO: u64 = 3;

/// 回收策略
///
/// 当前由 [`crate::logger`] 里的常量构造，日后可由设置界面覆盖。
#[derive(Clone, Copy, Debug)]
pub struct Policy {
    /// 文件年龄超过它即删除
    pub max_age: Duration,
    /// 目录总占用上限，超过后从最旧的文件开始删
    pub max_total: u64,
}

impl Policy {
    pub const fn new(max_age: Duration, max_total: u64) -> Self {
        Self { max_age, max_total }
    }

    /// 超量后的目标水位
    pub const fn target(&self) -> u64 {
        self.max_total / 4 * TARGET_RATIO
    }
}

/// 一次回收的结果
#[derive(Debug, Default)]
pub struct Report {
    /// 删除的 0 字节文件数
    pub removed_empty: usize,
    /// 删除的超龄文件数
    pub removed_expired: usize,
    /// 因超量删除的文件数
    pub removed_oversize: usize,
    /// 成功删除的总字节数（不含空文件）
    pub freed: u64,
    /// 清理后目录的占用
    pub remaining: u64,
    /// 删除失败的文件及原因
    pub failures: Vec<(PathBuf, String)>,
}

impl Report {
    /// 一共删了几个文件
    pub fn removed(&self) -> usize {
        self.removed_empty + self.removed_expired + self.removed_oversize
    }
}

/// 目录里的一个候选项
struct Candidate {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

/// 按策略清理一个目录
///
/// 顺序是 空文件 -> 超龄 -> 超量
pub fn prune(dir: &Path, policy: &Policy) -> Report {
    let mut report = Report::default();

    if let Err(e) = fs::create_dir_all(dir) {
        report.failures.push((dir.to_path_buf(), e.to_string()));
        return report;
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            report.failures.push((dir.to_path_buf(), e.to_string()));
            return report;
        }
    };

    let now = SystemTime::now();
    let mut candidates = Vec::new();

    for entry in entries.flatten() {
        // `file_type` 不跟随符号链接
        if !entry.file_type().map(|kind| kind.is_file()).unwrap_or(false) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else { continue };
        let size = metadata.len();
        let modified = metadata.modified().unwrap_or(now);

        // 不删除本次会话正在写的文件
        if size == 0 {
            match fs::remove_file(entry.path()) {
                Ok(()) => report.removed_empty += 1,
                Err(e) => report.failures.push((entry.path(), e.to_string())),
            }
            continue;
        }
        candidates.push(Candidate {
            path: entry.path(),
            size,
            modified,
        });
    }

    // 超龄
    let mut survivors = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        // 时钟回拨导致 mtime 在未来时 `duration_since` 返回 Err => 不删除，方向安全
        let expired = now
            .duration_since(candidate.modified)
            .map(|age| age > policy.max_age)
            .unwrap_or(false);
        if !expired {
            survivors.push(candidate);
            continue;
        }
        match fs::remove_file(&candidate.path) {
            Ok(()) => {
                report.removed_expired += 1;
                report.freed += candidate.size;
            }
            Err(e) => report.failures.push((candidate.path, e.to_string())),
        }
    }

    let total: u64 = survivors.iter().map(|candidate| candidate.size).sum();
    if total <= policy.max_total {
        report.remaining = total;
        return report;
    }

    // 超量时从最旧开始删。永不删最新的一个
    survivors.sort_by_key(|candidate| candidate.modified);
    let target = policy.target();
    let mut remaining = total;
    let keep_last = survivors.len().saturating_sub(1);

    for candidate in survivors.iter().take(keep_last) {
        if remaining <= target {
            break;
        }
        match fs::remove_file(&candidate.path) {
            Ok(()) => {
                report.removed_oversize += 1;
                report.freed += candidate.size;
                remaining -= candidate.size;
            }
            Err(e) => report.failures.push((candidate.path.clone(), e.to_string())),
        }
    }

    report.remaining = remaining;
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每个用例独占的临时目录
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rollcaller-retention-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, name: &str, bytes: usize) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, vec![b'x'; bytes]).unwrap();
        path
    }

    /// 把文件的 mtime 推到 `age` 之前
    fn backdate(path: &Path, age: Duration) {
        let when = SystemTime::now().checked_sub(age).unwrap();
        let file = fs::File::options().write(true).open(path).unwrap();
        file.set_times(fs::FileTimes::new().set_modified(when)).unwrap();
    }

    fn days(n: u64) -> Duration {
        Duration::from_secs(n * 24 * 60 * 60)
    }

    #[test]
    fn empty_files_are_removed_and_counted() {
        let dir = scratch("empty");
        let empty = write(&dir, "2026-11-01.log", 0);
        let kept = write(&dir, "2026-11-08.log", 16);

        let report = prune(&dir, &Policy::new(days(60), 1 << 20));

        assert_eq!(report.removed_empty, 1, "{report:?}");
        assert!(!empty.exists(), "0 字节文件应被删除");
        assert!(kept.exists(), "有内容的文件不该动");
        assert_eq!(report.freed, 0, "空文件不产生回收字节");
    }

    #[test]
    fn expired_files_are_removed() {
        let dir = scratch("expired");
        let old = write(&dir, "old.log", 100);
        let fresh = write(&dir, "fresh.log", 100);
        backdate(&old, days(61));
        backdate(&fresh, days(59));

        let report = prune(&dir, &Policy::new(days(60), 1 << 20));

        assert_eq!(report.removed_expired, 1, "{report:?}");
        assert!(!old.exists(), "61 天前的文件应被删除");
        assert!(fresh.exists(), "59 天前的文件应保留");
        assert_eq!(report.freed, 100);
        assert_eq!(report.remaining, 100);
    }

    #[test]
    fn oversize_removes_oldest_first_and_keeps_newest() {
        let dir = scratch("oversize");
        let oldest = write(&dir, "a.log", 1024);
        let middle = write(&dir, "b.log", 1024);
        let newest = write(&dir, "c.log", 1024);
        backdate(&oldest, Duration::from_secs(3 * 3600));
        backdate(&middle, Duration::from_secs(2 * 3600));
        backdate(&newest, Duration::from_secs(3600));

        // 上限 2048 ⇒ 目标 1536：3072 要删到 ≤1536，即删掉最旧的两个
        let report = prune(&dir, &Policy::new(days(60), 2048));

        assert_eq!(report.removed_oversize, 2, "{report:?}");
        assert!(!oldest.exists() && !middle.exists(), "最旧的两个应被删除");
        assert!(newest.exists(), "最新的一个必须保留");
        assert_eq!(report.remaining, 1024);
    }

    /// 只剩一个文件时即使超限也不删：它可能就是当前正在写的那个
    #[test]
    fn single_oversize_file_is_kept() {
        let dir = scratch("single");
        let only = write(&dir, "current.log", 4096);

        let report = prune(&dir, &Policy::new(days(60), 1024));

        assert_eq!(report.removed_oversize, 0, "{report:?}");
        assert!(only.exists(), "唯一的文件不得删除");
        assert_eq!(report.remaining, 4096, "超出上限的事实要上报给调用点");
    }

    #[test]
    fn directories_are_left_alone() {
        let dir = scratch("dirs");
        let nested = dir.join("2026-08");
        fs::create_dir_all(&nested).unwrap();
        let inner = write(&nested, "deep.log", 0);

        let report = prune(&dir, &Policy::new(days(60), 1 << 20));

        assert_eq!(report.removed(), 0, "不得递归进子目录：{report:?}");
        assert!(inner.exists(), "子目录里的文件不该被碰");
    }

    /// mtime 在未来（时钟回拨）时不删，方向安全
    #[test]
    fn future_timestamp_is_not_expired() {
        let dir = scratch("future");
        let path = write(&dir, "future.log", 64);
        let file = fs::File::options().write(true).open(&path).unwrap();
        file.set_times(fs::FileTimes::new().set_modified(SystemTime::now() + days(1)))
            .unwrap();

        let report = prune(&dir, &Policy::new(days(60), 1 << 20));

        assert_eq!(report.removed(), 0, "{report:?}");
        assert!(path.exists());
    }

    #[test]
    fn missing_directory_is_created_and_left_empty() {
        let dir = scratch("missing").join("nested");
        let report = prune(&dir, &Policy::new(days(60), 1 << 20));

        assert_eq!(report.removed(), 0);
        assert!(report.failures.is_empty(), "{report:?}");
        assert!(dir.is_dir(), "目录应被创建（appender 随后要用）");
    }
}
