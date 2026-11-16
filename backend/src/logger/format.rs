//! 日志行格式：时间戳、级别、线程名、目标与位置

use crate::util::time_utils::ISO8601;
use std::fmt;
use tracing_core::{Event, Subscriber};
use tracing_subscriber::{
    fmt::format::{FormatEvent, FormatFields, Writer},
    fmt::time::FormatTime,
    fmt::FmtContext,
    registry::LookupSpan,
};

/// ANSI 复位
const RESET: &str = "\x1b[0m";
/// 元信息的灰暗色
const DIM: &str = "\x1b[2m";

/// 日志时间戳
pub struct Timer;

impl FormatTime for Timer {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        write!(w, "{}", ISO8601(&jiff::Timestamp::now()))
    }
}

/// 一行日志
pub struct LogFormat;

impl<S, N> FormatEvent<S, N> for LogFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(&self, ctx: &FmtContext<'_, S, N>, mut writer: Writer<'_>, event: &Event<'_>) -> fmt::Result {
        let meta = event.metadata();

        Timer.format_time(&mut writer)?;
        writer.write_str(" ")?;

        let tag = format!("[{}]", meta.level().name());
        styled(&mut writer, meta.level().color(), &format!("{tag:<10}"))?;
        writer.write_str(" ")?;

        // 只写线程名足以区分线程
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

trait LevelInfo {
    /// 级别名称
    fn name(&self) -> &'static str;
    /// 级别配色
    fn color(&self) -> &'static str;
}

impl LevelInfo for tracing::Level {
    fn name(&self) -> &'static str {
        match *self {
            tracing::Level::ERROR => "ERROR",
            tracing::Level::WARN => "WARNING",
            tracing::Level::INFO => "INFO",
            tracing::Level::DEBUG => "DEBUG",
            tracing::Level::TRACE => "TRACE",
        }
    }

    fn color(&self) -> &'static str {
        match *self {
            tracing::Level::ERROR => "\x1b[31m", // 红
            tracing::Level::WARN => "\x1b[33m",  // 黄
            tracing::Level::INFO => "\x1b[32m",  // 绿
            tracing::Level::DEBUG => "\x1b[34m", // 蓝
            tracing::Level::TRACE => "\x1b[35m", // 紫
        }
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

/// `rollcaller_lib::logger::format` -> `rollcaller_lib`
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;
    use tracing_subscriber::layer::SubscriberExt;

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
        // 与 `logger::build` 里的层用同一套配置，只把 writer 换成内存缓冲
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .event_format(LogFormat)
                .with_ansi(ansi)
                .with_writer(sink.clone()),
        );
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
        assert!(line.contains("src/logger/format.rs:"), "位置应为正斜杠相对路径：{line}");
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
        assert_eq!(tracing::Level::WARN.name(), "WARNING");
        assert_eq!(tracing::Level::ERROR.name(), "ERROR");
        assert_eq!(tracing::Level::INFO.name(), "INFO");
    }

    #[test]
    fn target_is_reduced_to_crate() {
        assert_eq!(crate_name("rollcaller_lib::logger::format"), "rollcaller_lib");
        assert_eq!(crate_name("tauri_plugin_updater::updater"), "tauri_plugin_updater");
        assert_eq!(crate_name(""), "");
    }

    #[test]
    fn file_paths_are_normalized_and_trimmed() {
        assert_eq!(format_file("src\\logger\\format.rs"), "src/logger/format.rs");

        assert_eq!(
            format_file("/home/x/.config/.cargo/registry/src/index.crates.io-19491001/lib_name/src/rust_file.rs"),
            "lib_name/src/rust_file.rs"
        );
    }
}
