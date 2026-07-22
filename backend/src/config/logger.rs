use crate::config::app_paths as AppPaths;
use tracing::metadata::LevelFilter;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_log::LogTracer;
use tracing_subscriber::{
    fmt::format, fmt::layer, fmt::time::FormatTime, layer::SubscriberExt, Layer,
};

/// 自定义时间格式：Unix 毫秒时间戳
struct UnixTimestampMs;

#[cfg(not(debug_assertions))]
/// 遥测JSON日志过滤器
/// 只有设置了 telemetry = true 的才能被接收并等待发送
struct TelemetryFilter;

impl FormatTime for UnixTimestampMs {
    fn format_time(&self, w: &mut format::Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", jiff::Timestamp::now().as_millisecond())
    }
}

#[cfg(not(debug_assertions))]
impl<S> Filter<S> for TelemetryFilter {
    fn enabled(
        &self,
        metadata: &tracing_core::Metadata<'_>,
        _: &tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        metadata.level() >= &tracing::Level::ERROR
    }

    fn event_enabled(
        &self,
        _event: &Event<'_>,
        _: &tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        false
    }
}

/// 初始化全局日志订阅器
pub fn build() -> impl tracing::Subscriber {
    let file_log_dir = AppPaths::logs_dir().join("f");
    let _ = std::fs::create_dir_all(&file_log_dir);

    // 开发/生产环境控制台日志级别
    #[cfg(debug_assertions)]
    let console_level = LevelFilter::TRACE;
    #[cfg(not(debug_assertions))]
    let console_level = LevelFilter::INFO;

    let console_layer = layer()
        .with_ansi(true)
        .with_line_number(true)
        .with_file(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_span_events(format::FmtSpan::ENTER | format::FmtSpan::CLOSE)
        .with_writer(std::io::stdout)
        .with_filter(console_level);

    let file_layer = layer()
        .with_ansi(false)
        .with_line_number(true)
        .with_file(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_writer(
            RollingFileAppender::builder()
                .filename_suffix("log")
                .rotation(Rotation::WEEKLY)
                .build(&file_log_dir)
                .expect("Failed to initialize rolling file appender."),
        )
        .with_filter(LevelFilter::WARN);

    // JSON 层
    #[cfg(not(debug_assertions))]
    let json_layer = {
        let json_log_dir = AppPaths::logs_dir().join("telemetry").as_path();
        let _ = std::fs::create_dir_all(json_log_dir);

        layer()
            .json()
            .with_ansi(false)
            .with_line_number(true)
            .with_file(true)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_timer(UnixTimestampMs)
            .with_writer(
                RollingFileAppender::builder()
                    .filename_suffix("jsonl")
                    .rotation(Rotation::DAILY)
                    .build(json_log_dir)
                    .expect("Failed to initialize rolling file appender."),
            )
            .with_filter(TelemetryFilter)
    };

    let subscriber = tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer);

    #[cfg(not(debug_assertions))]
    let subscriber = subscriber.with(json_layer);

    subscriber
}

pub fn init() {
    let subscriber = build();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set up global log subscriber.");

    LogTracer::init().expect("Failed to bridge log to tracing.");
}
