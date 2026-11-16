//! 应用日志

pub mod format;
pub mod policy;
pub mod retention;

use crate::config::app_paths;
use policy::LoggerPolicy;
use retention::RetentionPolicy;
use std::path::PathBuf;
use tracing::metadata::LevelFilter;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_core::Subscriber;
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::layer, layer::SubscriberExt, Layer};

/// 文本日志目录
pub fn text_dir() -> PathBuf {
    app_paths::logs_dir().join("f")
}

/// JSON 遥测目录
pub fn telemetry_dir() -> PathBuf {
    app_paths::logs_dir().join("telemetry")
}

/// 构建全局日志订阅器
pub fn build(policy: &LoggerPolicy) -> impl Subscriber {
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
                .with_filter(policy.level.filter()),
        ),
        Err(e) => {
            eprintln!(
                "[warn] 文件日志初始化失败：{e}，本次运行只输出到控制台：{}",
                file_log_dir.display()
            );
            None
        }
    };

    // JSON 遥测层。
    // 1. 只收 `ERROR`
    // 2. 过滤器用标准 `LevelFilter::ERROR`
    // 3. 接收端尚未就绪，`telemetry` feature 默认不参与编译
    #[cfg(any())]
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
                    .with_filter(LevelFilter::ERROR),
            ),
            Err(e) => {
                eprintln!("[warn] 日志初始化失败：{e}");
                None
            }
        }
    };

    let subscriber = tracing_subscriber::registry().with(console_layer).with(file_layer);

    #[cfg(feature = "telemetry")]
    let subscriber = subscriber.with(json_layer);

    subscriber
}

/// 安装全局订阅器，并回收过期日志
pub fn init() {
    // `None` = 尚无持久化配置；设置系统落地后这里是 `Some(settings.logger)`
    let policy = LoggerPolicy::resolve(None);

    // 回收必须在 `build()` 之前，appender 一旦构造就会创建 0 字节当期文件，
    // 之后回收会把它当空文件删掉；而 Rust 在 Windows 上采用 `FILE_SHARE_DELETE`
    // 的方式打开文件，删除会成功，但后续写入静默丢失。
    let text_report = retention::prune(&text_dir(), &policy.retention);
    let telemetry_report = retention::prune(&telemetry_dir(), &RetentionPolicy::TELEMETRY);

    tracing::subscriber::set_global_default(build(&policy)).expect("Failed to set up global log subscriber.");
    LogTracer::init().expect("Failed to bridge log to tracing.");

    // 订阅器装好后报告
    report("f", &text_report, &policy.retention);
    report("telemetry", &telemetry_report, &RetentionPolicy::TELEMETRY);

    tracing::info!(
        level = %policy.level,
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
fn report(label: &str, report: &retention::Report, policy: &RetentionPolicy) {
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
            "日志清理后仍超出上限，除当前文件外已无内容可删"
        );
    }
}
