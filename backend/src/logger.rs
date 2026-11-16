//! 应用日志

pub mod format;
pub mod policy;
pub mod retention;

use crate::config::app_paths;
use policy::{LogLevel, LoggerPolicy};
use retention::RetentionPolicy;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing::metadata::LevelFilter;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_core::Subscriber;
use tracing_log::LogTracer;
use tracing_subscriber::registry::Registry;
use tracing_subscriber::{fmt::layer, layer::SubscriberExt, reload, Layer};

/// 文件层的级别句柄
pub type LevelHandle = reload::Handle<LevelFilter, Registry>;

/// 运行期可改的文件层句柄
static LEVEL_HANDLE: OnceLock<Option<LevelHandle>> = OnceLock::new();

/// 文本日志目录
pub fn text_dir() -> PathBuf {
    app_paths::logs_dir().join("f")
}

/// JSON 遥测目录
pub fn telemetry_dir() -> PathBuf {
    app_paths::logs_dir().join("telemetry")
}

/// 构建全局日志订阅器，并返回文件层的级别句柄
///
/// 返回 `None` 表示文件层没建出来，降级为只输出控制台，此时句柄不存在。
pub fn build(policy: &LoggerPolicy) -> (impl Subscriber, Option<LevelHandle>) {
    // 开发/生产环境控制台日志级别
    #[cfg(debug_assertions)]
    let console_level = LevelFilter::DEBUG;
    #[cfg(not(debug_assertions))]
    let console_level = LevelFilter::INFO;

    build_at(&text_dir(), policy, crate::console::has_output().then_some(console_level))
}

/// [`build`] 的可测试部分
fn build_at(dir: &Path, policy: &LoggerPolicy, console: Option<LevelFilter>) -> (impl Subscriber, Option<LevelHandle>) {
    let console_layer = console.map(|level| {
        layer()
            .event_format(format::LogFormat)
            .with_ansi(crate::console::ansi_enabled())
            .with_writer(std::io::stdout)
            .with_filter(level)
    });

    let _ = std::fs::create_dir_all(dir);

    // 句柄内部是指向 filter 的 `Weak`：filter 必须活到进程末尾随 layer 挂在全局订阅器里，
    // 句柄才不会在 `modify` 时报 `SubscriberGone`
    let (file_filter, level_handle) = reload::Layer::<LevelFilter, Registry>::new(policy.level.filter());
    let file_layer = match RollingFileAppender::builder()
        .filename_suffix("log")
        .rotation(Rotation::WEEKLY)
        .build(dir)
    {
        Ok(appender) => Some(
            layer()
                .event_format(format::LogFormat)
                .with_ansi(false)
                .with_writer(appender)
                .with_filter(file_filter),
        ),
        Err(e) => {
            eprintln!("[warn] 文件日志初始化失败：{e}，本次运行只输出到控制台：{}", dir.display());
            None
        }
    };
    let file_ready = file_layer.is_some();

    // JSON 遥测层。
    // 1. 只收 `ERROR`
    // 2. 过滤器用标准 `LevelFilter::ERROR`
    // 3. 接收端尚未就绪，不参与编译
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

    // 文件层必须第一个挂，理由见 [`LevelHandle`]
    let subscriber = tracing_subscriber::registry().with(file_layer).with(console_layer);

    #[cfg(any())]
    let subscriber = subscriber.with(json_layer);

    (subscriber, file_ready.then_some(level_handle))
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

    let (subscriber, level_handle) = build(&policy);
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set up global log subscriber.");
    // 句柄本身只有 `Weak`，真正续命的是挂在全局订阅器上的那个 layer
    let _ = LEVEL_HANDLE.set(level_handle);
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

/// 运行期切换文件层级别
pub fn set_level(level: LogLevel) -> Result<(), String> {
    let handle = LEVEL_HANDLE
        .get()
        .ok_or("日志系统尚未初始化")?
        .as_ref()
        .ok_or("文件日志不可用，级别切换未生效")?;

    // 先记录，再切换，此条日志记录是否落盘，由切换前的级别决定
    // 记录后，切到 `warn` / `error` 会留下痕迹，
    let from = handle
        .clone_current()
        .map_or_else(|| "?".to_string(), |filter| filter.to_string());
    tracing::info!(from = %from, to = %level, "切换文件日志级别");

    handle
        .modify(|filter| *filter = level.filter())
        .map_err(|e| format!("切换日志级别失败：{e}"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 每个用例独占的临时目录
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rollcaller-logger-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 目录里唯一的日志文件（WEEKLY 轮转当期只有一个）
    fn log_text(dir: &Path) -> String {
        let path = fs::read_dir(dir)
            .unwrap()
            .flatten()
            .find(|entry| entry.file_type().map(|kind| kind.is_file()).unwrap_or(false))
            .expect("appender 应在构造时创建当期文件")
            .path();
        fs::read_to_string(path).unwrap()
    }

    /// 运行期改级别立即生效，含最容易坏的方向（由紧到松，必须重建 interest 缓存）
    #[test]
    fn reload_takes_effect_at_runtime() {
        let dir = scratch("reload");
        // 刻意不给控制台层：这条用例只验证文件层，输出也不会混进测试日志
        let (subscriber, handle) = build_at(&dir, &LoggerPolicy::default(), None);
        let handle = handle.expect("临时目录应能建出文件层");
        let dispatch = tracing::Dispatch::new(subscriber);

        tracing::dispatcher::with_default(&dispatch, || {
            tracing::info!("切换前-可见");
            tracing::debug!("切换前-不可见");
        });

        handle.modify(|filter| *filter = LevelFilter::DEBUG).unwrap();

        tracing::dispatcher::with_default(&dispatch, || {
            tracing::debug!("切换后-可见");
        });

        let text = log_text(&dir);
        assert!(text.contains("切换前-可见"), "默认 INFO 下 INFO 应落盘：{text}");
        assert!(!text.contains("切换前-不可见"), "默认 INFO 下 DEBUG 不该落盘：{text}");
        assert!(text.contains("切换后-可见"), "调到 DEBUG 后 DEBUG 应落盘：{text}");
    }
}
