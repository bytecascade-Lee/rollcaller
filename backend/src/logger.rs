//! 应用日志

pub mod format;
pub mod retention;

use crate::config::app_paths;
use retention::Policy;
use std::path::PathBuf;
use std::time::Duration;
use tracing::metadata::LevelFilter;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_core::Subscriber;
use tracing_log::LogTracer;
#[cfg(not(debug_assertions))]
use tracing_subscriber::layer::Filter;
use tracing_subscriber::{fmt::layer, layer::SubscriberExt, Layer};

/// 覆盖文件层级别的环境变量，取值见 [`parse_level`]
pub const FILE_LOG_LEVEL_ENV: &str = "ROLLCALLER_FILE_LOG_LEVEL";

/// 一天的秒数，供保留期表达式使用
const DAY: u64 = 24 * 60 * 60;

/// 文本日志：保留 60 天，总量上限 32 MiB
///
/// 按预设的一月约 250 KB，32 MiB 是 128 倍余量，
/// 预防某处逐帧写日志等畸形增长，而非日常配额
const TEXT_POLICY: Policy = Policy::new(Duration::from_secs(60 * DAY), 32 * 1024 * 1024);

/// 遥测日志只收 `ERROR`
const TELEMETRY_POLICY: Policy = Policy::new(Duration::from_secs(60 * DAY), 16 * 1024 * 1024);

#[cfg(not(debug_assertions))]
struct TelemetryFilter;

#[cfg(not(debug_assertions))]
impl<S> Filter<S> for TelemetryFilter {
    fn enabled(&self, metadata: &tracing_core::Metadata<'_>, _: &tracing_subscriber::layer::Context<'_, S>) -> bool {
        metadata.level() >= &tracing::Level::ERROR
    }

    fn event_enabled(&self, _event: &tracing_core::Event<'_>, _: &tracing_subscriber::layer::Context<'_, S>) -> bool {
        false
    }
}

/// 文本日志目录
pub fn text_dir() -> PathBuf {
    app_paths::logs_dir().join("f")
}

/// JSON 遥测目录
pub fn telemetry_dir() -> PathBuf {
    app_paths::logs_dir().join("telemetry")
}

/// 解析日志级别字符串：`off` / `error` / `warn` / `info` / `debug` / `trace` 与 `0..5`，大小写不敏感
///
/// `LevelFilter::FromStr` 把 `""` 解析为 `ERROR`，显式忽略空字符串，
/// 避免意外将日志等级设为 `ERROR`，从而只剩错误
pub fn parse_level(raw: &str) -> Option<LevelFilter> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    normalized.parse().ok()
}

/// 文件层级别默认 `INFO`，可用 [`FILE_LOG_LEVEL_ENV`] 覆盖；非法取值同样回落到 `INFO`
pub fn file_level() -> LevelFilter {
    std::env::var(FILE_LOG_LEVEL_ENV)
        .ok()
        .and_then(|raw| parse_level(&raw))
        .unwrap_or(LevelFilter::INFO)
}

/// 构建全局日志订阅器
pub fn build() -> impl Subscriber {
    // 开发/生产环境控制台日志级别
    #[cfg(debug_assertions)]
    let console_level = LevelFilter::DEBUG;
    #[cfg(not(debug_assertions))]
    let console_level = LevelFilter::INFO;

    let console_layer = crate::console::has_output().then(|| {
        layer()
            .event_format(format::LogFormat)
            .with_ansi(crate::console::ansi_enabled())
            .with_writer(std::io::stdout)
            .with_filter(console_level)
    });

    let file_log_dir = text_dir();
    let _ = std::fs::create_dir_all(&file_log_dir);
    let file_layer = match RollingFileAppender::builder()
        .filename_suffix("log")
        .rotation(Rotation::WEEKLY)
        .build(&file_log_dir)
    {
        Ok(appender) => Some(
            layer()
                .event_format(format::LogFormat)
                .with_ansi(false)
                .with_writer(appender)
                .with_filter(file_level()),
        ),
        Err(e) => {
            eprintln!(
                "[warn] 文件日志初始化失败：{e}，本次运行只输出到控制台：{}",
                file_log_dir.display()
            );
            None
        }
    };

    // JSON 层
    #[cfg(not(debug_assertions))]
    let json_layer = {
        let json_log_dir = telemetry_dir();
        let _ = std::fs::create_dir_all(&json_log_dir);

        match RollingFileAppender::builder()
            .filename_suffix("jsonl")
            .rotation(Rotation::DAILY)
            .build(&json_log_dir)
        {
            Ok(appender) => Some(
                layer()
                    .json()
                    .with_ansi(false)
                    .with_line_number(true)
                    .with_file(true)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_timer(format::Timer)
                    .with_writer(appender)
                    .with_filter(TelemetryFilter),
            ),
            Err(e) => {
                eprintln!("[warn] 日志初始化失败：{e}");
                None
            }
        }
    };

    let subscriber = tracing_subscriber::registry().with(console_layer).with(file_layer);

    #[cfg(not(debug_assertions))]
    let subscriber = subscriber.with(json_layer);

    subscriber
}

/// 安装全局订阅器，并回收过期日志
pub fn init() {
    // 回收必须在 `build()` 之前，appender 一旦构造就会创建 0 字节当期文件，
    // 之后回收会把它当空文件删掉；而 Rust 在 Windows 上采用 `FILE_SHARE_DELETE`
    // 的方式打开文件，删除会成功，但后续写入静默丢失。
    let text_report = retention::prune(&text_dir(), &TEXT_POLICY);
    let telemetry_report = retention::prune(&telemetry_dir(), &TELEMETRY_POLICY);

    tracing::subscriber::set_global_default(build()).expect("Failed to set up global log subscriber.");
    LogTracer::init().expect("Failed to bridge log to tracing.");

    // 订阅器装好后报告
    report("f", &text_report, &TEXT_POLICY);
    report("telemetry", &telemetry_report, &TELEMETRY_POLICY);

    tracing::info!(
        level = %file_level(),
        dir = %text_dir().display(),
        "日志已就绪"
    );

    // panic 兜底，默认 hook 只写 stderr，而 release 没有控制台
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("线程 panic：{info}");
        // 保留默认行为，开发期仍有 stderr 与 backtrace
        previous(info);
    }));
}

/// 记录一次回收，如果未删除文件且未失败，则完全静默
fn report(label: &str, report: &retention::Report, policy: &Policy) {
    if report.removed() > 0 {
        tracing::info!(
            dir = label,
            empty = report.removed_empty,
            expired = report.removed_expired,
            oversize = report.removed_oversize,
            freed = report.freed,
            "已清理日志文件"
        );
    }
    for (path, error) in &report.failures {
        tracing::warn!(path = %path.display(), "删除日志文件失败：{error}");
    }
    if report.remaining > policy.max_total {
        tracing::warn!(
            dir = label,
            remaining = report.remaining,
            limit = policy.max_total,
            "日志清理后仍超出上限（除当前文件外已无内容可删）"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_parsing_accepts_names_numbers_and_case() {
        assert_eq!(parse_level("debug"), Some(LevelFilter::DEBUG));
        assert_eq!(parse_level("  INFO  "), Some(LevelFilter::INFO));
        assert_eq!(parse_level("Trace"), Some(LevelFilter::TRACE));
        assert_eq!(parse_level("off"), Some(LevelFilter::OFF));
        assert_eq!(parse_level("4"), Some(LevelFilter::DEBUG));
        assert_eq!(parse_level("0"), Some(LevelFilter::OFF));
    }

    /// 空值与乱码均回退到 `None`，不得被当成某个级别
    #[test]
    fn level_parsing_rejects_empty_and_garbage() {
        assert_eq!(parse_level(""), None);
        assert_eq!(parse_level("   "), None);
        assert_eq!(parse_level("verbose"), None);
        assert_eq!(parse_level("99"), None);
    }
}
