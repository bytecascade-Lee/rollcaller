use crate::config::app_paths as AppPaths;
use crate::util::time_utils::ISO8601;
use std::fmt;
use tracing::metadata::LevelFilter;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_core::{Event, Subscriber};
use tracing_log::LogTracer;
#[cfg(not(debug_assertions))]
use tracing_subscriber::layer::Filter;
use tracing_subscriber::{
    fmt::format::{FormatEvent, FormatFields, Writer},
    fmt::time::FormatTime,
    fmt::{layer, FmtContext},
    layer::SubscriberExt,
    registry::LookupSpan,
    Layer,
};

/// ANSI 复位
const RESET: &str = "\x1b[0m";
/// 元信息（时间、线程名、位置）的灰暗色
const DIM: &str = "\x1b[2m";

/// 日志时间戳
struct Timer;

impl FormatTime for Timer {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        write!(w, "{}", ISO8601(&jiff::Timestamp::now()))
    }
}

/// 一行日志
struct LogFormat;

impl<S, N> FormatEvent<S, N> for LogFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(&self, ctx: &FmtContext<'_, S, N>, mut writer: Writer<'_>, event: &Event<'_>) -> fmt::Result {
        let meta = event.metadata();

        Timer.format_time(&mut writer)?;
        writer.write_str(" ")?;

        let tag = format!("[{}]", level_name(meta.level()));
        styled(&mut writer, level_color(meta.level()), &format!("{tag:<10}"))?;
        writer.write_str(" ")?;

        // 只写线程名（`main` / `tokio-rt-worker`），足以区分线程；不带 ThreadId
        if let Some(thread) = std::thread::current().name() {
            styled(&mut writer, DIM, thread)?;
            writer.write_str(" ")?;
        }

        let target = crate_name(meta.target());
        if !target.is_empty() {
            styled(&mut writer, DIM, target)?;
            writer.write_str(" ")?;
        }

        if let Some(file) = meta.file() {
            let path = format_file(file);
            let place = match meta.line() {
                Some(line) => format!("{path}:{line}"),
                None => path,
            };
            styled(&mut writer, DIM, &place)?;
            writer.write_str(": ")?;
        }

        ctx.format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

/// 输出一段可能带色的文本
fn styled(writer: &mut Writer<'_>, style: &str, text: &str) -> fmt::Result {
    if writer.has_ansi_escapes() {
        write!(writer, "{style}{text}{RESET}")
    } else {
        writer.write_str(text)
    }
}

/// 级别名称
fn level_name(level: &tracing::Level) -> &'static str {
    match *level {
        tracing::Level::ERROR => "ERROR",
        tracing::Level::WARN => "WARNING",
        tracing::Level::INFO => "INFO",
        tracing::Level::DEBUG => "DEBUG",
        tracing::Level::TRACE => "TRACE",
    }
}

/// 级别配色
fn level_color(level: &tracing::Level) -> &'static str {
    match *level {
        tracing::Level::ERROR => "\x1b[31m", // 红
        tracing::Level::WARN => "\x1b[33m",  // 黄
        tracing::Level::INFO => "\x1b[32m",  // 绿
        tracing::Level::DEBUG => "\x1b[34m", // 蓝
        tracing::Level::TRACE => "\x1b[35m", // 紫
    }
}

/// `rollcaller_lib::config::logger` -> `rollcaller_lib`
fn crate_name(target: &str) -> &str {
    target.split("::").next().unwrap_or_default()
}

/// 位置路径
fn format_file(file: &str) -> String {
    const REGISTRY: &str = ".cargo/registry/src/";
    let normalized = file.replace('\\', "/");
    let Some(rest) = normalized.find(REGISTRY).map(|at| &normalized[at + REGISTRY.len()..]) else {
        return normalized;
    };

    match rest.split_once('/') {
        Some((_, tail)) => tail.to_owned(),
        None => rest.to_owned(),
    }
}

#[cfg(not(debug_assertions))]
struct TelemetryFilter;

#[cfg(not(debug_assertions))]
impl<S> Filter<S> for TelemetryFilter {
    fn enabled(&self, metadata: &tracing_core::Metadata<'_>, _: &tracing_subscriber::layer::Context<'_, S>) -> bool {
        metadata.level() >= &tracing::Level::ERROR
    }

    fn event_enabled(&self, _event: &Event<'_>, _: &tracing_subscriber::layer::Context<'_, S>) -> bool {
        false
    }
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
            .event_format(LogFormat)
            .with_ansi(crate::console::ansi_enabled())
            .with_writer(std::io::stdout)
            .with_filter(console_level)
    });

    let file_log_dir = AppPaths::logs_dir().join("f");
    let _ = std::fs::create_dir_all(&file_log_dir);
    let file_layer = match RollingFileAppender::builder()
        .filename_suffix("log")
        .rotation(Rotation::WEEKLY)
        .build(&file_log_dir)
    {
        Ok(appender) => Some(
            layer()
                .event_format(LogFormat)
                .with_ansi(false)
                .with_writer(appender)
                .with_filter(LevelFilter::WARN),
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
        let json_log_dir = AppPaths::logs_dir().join("telemetry");
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
                    .with_timer(Timer)
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

pub fn init() {
    let subscriber = build();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set up global log subscriber.");

    LogTracer::init().expect("Failed to bridge log to tracing.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for Sink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Sink {
        type Writer = Sink;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    /// 在 `LogFormat` 下渲染一条事件，返回整行
    fn render(ansi: bool, event: impl FnOnce()) -> String {
        let sink = Sink::default();
        // 与 `build()` 里的层用同一套配置，只把 writer 换成内存缓冲
        let subscriber =
            tracing_subscriber::registry().with(layer().event_format(LogFormat).with_ansi(ansi).with_writer(sink.clone()));
        tracing::subscriber::with_default(subscriber, event);
        let bytes = sink.0.lock().unwrap().clone();
        String::from_utf8(bytes).unwrap()
    }

    /// 三层共用的 timer 必须产出规范串
    #[test]
    fn timer_writes_canonical_iso8601() {
        let mut text = String::new();
        let mut writer = Writer::new(&mut text);
        Timer.format_time(&mut writer).unwrap();

        assert_eq!(text.len(), 24, "应为定长 ISO 8601 串，实得 {text:?}");
        assert_eq!(&text[19..20], ".", "小数位必须存在：{text}");
        assert!(text.ends_with('Z'), "必须是无偏移的 UTC：{text}");
        std::str::FromStr::from_str(&text).map(|_: jiff::Timestamp| ()).unwrap();
    }

    /// 整行形状
    #[test]
    fn line_shape() {
        let line = render(false, || tracing::warn!(student = 1, "already exists"));

        assert!(line.starts_with("20"), "{line}");
        assert_eq!(&line[24..25], " ", "时间后应紧跟一个空格：{line}");
        assert!(line.contains("[WARNING]"), "级别应写作 WARNING 并带方括号：{line}");
        assert!(!line.contains("[WARN]"), "不应出现 Rust 的短名：{line}");

        let thread = std::thread::current().name().unwrap_or_default().to_owned();
        assert!(!thread.is_empty(), "测试线程应带名字");
        assert!(line.contains(&thread), "应标明线程名：{line}");
        assert!(!line.contains("ThreadId"), "只要线程名，不要 ThreadId：{line}");
        assert!(line.contains(" rollcaller_lib "), "target 应只留 crate 段：{line}");
        assert!(line.contains("src/config/logger.rs:"), "位置应为正斜杠相对路径：{line}");
        assert!(line.contains("already exists"), "正文应完整：{line}");
        assert!(line.contains("student=1"), "字段应保留：{line}");
        assert!(line.ends_with('\n'), "每行以换行收尾：{line:?}");
    }

    /// 线程名排在级别之后、crate 之前，且不带 ThreadId
    #[test]
    fn thread_name_is_marked() {
        let line = std::thread::Builder::new()
            .name("tokio-rt-worker".into())
            .spawn(|| render(false, || tracing::warn!("x")))
            .unwrap()
            .join()
            .unwrap();

        assert!(line.contains("[WARNING]  tokio-rt-worker rollcaller_lib "), "{line}");
        assert!(!line.contains("ThreadId"), "只要线程名，不要 ThreadId：{line}");
    }

    /// 控制台带色时级别被彩色包裹；文件层不上色
    #[test]
    fn ansi_only_on_demand() {
        let colored = render(true, || tracing::warn!("x"));
        assert!(colored.contains("\x1b[33m[WARNING]"), "WARN 应为黄色：{colored:?}");

        let plain = render(false, || tracing::warn!("x"));
        assert!(!plain.contains('\x1b'), "无色输出不应含转义序列：{plain:?}");
    }

    #[test]
    fn level_names() {
        assert_eq!(level_name(&tracing::Level::WARN), "WARNING");
        assert_eq!(level_name(&tracing::Level::ERROR), "ERROR");
        assert_eq!(level_name(&tracing::Level::INFO), "INFO");
    }

    #[test]
    fn target_is_reduced_to_crate() {
        assert_eq!(crate_name("rollcaller_lib::config::logger"), "rollcaller_lib");
        assert_eq!(crate_name("tauri_plugin_updater::updater"), "tauri_plugin_updater");
        assert_eq!(crate_name(""), "");
    }

    #[test]
    fn file_paths_are_normalized_and_trimmed() {
        assert_eq!(format_file("src\\config\\logger.rs"), "src/config/logger.rs");

        assert_eq!(
            format_file("/home/x/.config/.cargo/registry/src/index.crates.io-19491001/lib_name/src/rust_file.rs"),
            "lib_name/src/rust_file.rs"
        );
    }
}
