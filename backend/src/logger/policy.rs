//! 日志可配项

use crate::logger::retention::RetentionPolicy;
use std::fmt;
use tracing::metadata::LevelFilter;

/// 覆盖文件层级别的环境变量，取值见 [`LogLevel::parse`]
pub const FILE_LOG_LEVEL_ENV: &str = "ROLLCALLER_FILE_LOG_LEVEL";

/// 用户可配的日志级别
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogLevel(LevelFilter);

impl LogLevel {
    /// 默认级别
    pub const INFO: Self = Self(LevelFilter::INFO);

    /// 解析 `off` / `error` / `warn` / `info` / `debug` / `trace` 与 `0..5`，大小写不敏感
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        trimmed.parse().ok().map(Self)
    }

    /// 交给 `tracing` 的过滤器
    pub const fn filter(self) -> LevelFilter {
        self.0
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::INFO
    }
}

/// 小写口径（`off` / `error` / `warn` / `info` / `debug` / `trace`）
impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// 日志的全部可配项
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoggerPolicy {
    /// 文件层级别
    ///
    /// 注：控制台日志等级和遥测日志等级不允许用户配置
    pub level: LogLevel,
    /// 文本日志目录的回收策略
    ///
    /// 注：遥测目录固定按 [`RetentionPolicy::TELEMETRY`] 回收
    pub retention: RetentionPolicy,
}

impl Default for LoggerPolicy {
    fn default() -> Self {
        Self {
            level: LogLevel::INFO,
            retention: RetentionPolicy::TEXT,
        }
    }
}

impl LoggerPolicy {
    /// 优先级：**环境变量 > 持久化配置 > 默认**
    pub fn resolve(persisted: Option<Self>) -> Self {
        Self::resolve_from(persisted, std::env::var(FILE_LOG_LEVEL_ENV).ok().as_deref())
    }

    /// [`Self::resolve`] 的纯函数部分，`env` 即环境变量原值
    fn resolve_from(persisted: Option<Self>, env: Option<&str>) -> Self {
        let base = persisted.unwrap_or_default();
        Self {
            // 环境变量只管级别：回收策略来自持久化配置，这里不得连带覆盖
            level: env.and_then(LogLevel::parse).unwrap_or(base.level),
            retention: base.retention,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_accepts_names_numbers_and_case() {
        assert_eq!(LogLevel::parse("debug"), Some(LogLevel(LevelFilter::DEBUG)));
        assert_eq!(LogLevel::parse("  INFO  "), Some(LogLevel(LevelFilter::INFO)));
        assert_eq!(LogLevel::parse("Trace"), Some(LogLevel(LevelFilter::TRACE)));
        assert_eq!(LogLevel::parse("off"), Some(LogLevel(LevelFilter::OFF)));
        assert_eq!(LogLevel::parse("4"), Some(LogLevel(LevelFilter::DEBUG)));
        assert_eq!(LogLevel::parse("0"), Some(LogLevel(LevelFilter::OFF)));
    }

    /// 空值与乱码均回退到 `None`，不得被当成某个级别
    #[test]
    fn parsing_rejects_empty_and_garbage() {
        assert_eq!(LogLevel::parse(""), None);
        assert_eq!(LogLevel::parse("   "), None);
        assert_eq!(LogLevel::parse("verbose"), None);
        assert_eq!(LogLevel::parse("99"), None);
    }

    /// 配置里读出来再写回去必须同值，否则设置界面会把用户的输入改掉
    #[test]
    fn display_round_trips_through_parse() {
        for raw in ["off", "error", "warn", "info", "debug", "trace"] {
            let level = LogLevel::parse(raw).unwrap();
            assert_eq!(level.to_string(), raw);
            assert_eq!(LogLevel::parse(&level.to_string()), Some(level));
        }
    }

    /// 环境变量只覆盖级别；空值与乱码回落，不得把回收策略一起改掉
    #[test]
    fn env_overrides_level_only() {
        let persisted = LoggerPolicy {
            level: LogLevel::parse("warn").unwrap(),
            retention: RetentionPolicy::TELEMETRY,
        };

        let resolved = LoggerPolicy::resolve_from(Some(persisted), Some("debug"));
        assert_eq!(resolved.level, LogLevel::parse("debug").unwrap());
        assert_eq!(resolved.retention, RetentionPolicy::TELEMETRY);

        for raw in [None, Some(""), Some("  "), Some("verbose")] {
            assert_eq!(
                LoggerPolicy::resolve_from(Some(persisted), raw).level,
                persisted.level,
                "raw={raw:?} 应回落到持久化配置"
            );
        }
        assert_eq!(LoggerPolicy::resolve_from(None, None), LoggerPolicy::default());
    }
}
