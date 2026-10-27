//! CLI 参数解析（纯函数：不读环境、不做 IO，参数由调用方传入，便于单测）
//!
//! # 规则
//!
//! - **命令互斥**：一次只做一件事，同时给两个命令是用法错误（旧实现取「第一个命中」，
//!   静默吞掉冲突）；
//! - **`--json` 是修饰符**：位置任意、重复出现无害；
//! - **`-h/--help` 优先**：出现即返回，忽略其它参数（`rollcaller -v --help` 打帮助）；
//! - **非 `-` 开头的参数一律忽略**：更新重启会把原启动参数经 NSIS `/ARGS` 转发回来，
//!   对位置参数报错会变成「更新后应用起不来」；
//! - **拼错的参数报错**：`--versionn` 以前会静默启动图形界面，现在退 2。

use std::fmt;

/// CLI 命令（互斥）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    BuildInfo,
    AppPaths,
    Mode,
    Doctor,
}

impl Command {
    /// 从命令行标志识别命令；不是命令返回 `None`
    fn from_flag(flag: &str) -> Option<Self> {
        match flag {
            "-h" | "--help" => Some(Self::Help),
            "-v" | "--version" => Some(Self::Version),
            "--build-info" => Some(Self::BuildInfo),
            "--app-paths" => Some(Self::AppPaths),
            "--mode" => Some(Self::Mode),
            "--doctor" => Some(Self::Doctor),
            _ => None,
        }
    }

    /// 规范写法（错误信息里用，不引用用户实际敲的别名）
    pub fn flag(self) -> &'static str {
        match self {
            Self::Help => "--help",
            Self::Version => "--version",
            Self::BuildInfo => "--build-info",
            Self::AppPaths => "--app-paths",
            Self::Mode => "--mode",
            Self::Doctor => "--doctor",
        }
    }
}

/// 输出格式
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// 人类可读（单值裸输出；多字段对齐的 `key : value`）
    Human,
    /// 单个 JSON 对象
    Json,
}

/// 一次完整的 CLI 调用
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Invocation {
    pub command: Command,
    pub format: Format,
}

/// 用法错误（调用方负责 `exit(2)`）
#[derive(Debug, PartialEq, Eq)]
pub enum UsageError {
    /// 未知的 `-` 开头参数
    Unknown(String),
    /// 同时指定了两个命令
    Conflict(Command, Command),
    /// 只给了 `--json`，没有命令
    MissingCommand,
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(arg) => write!(f, "未知参数 `{arg}`"),
            Self::Conflict(first, second) => {
                write!(f, "命令不能同时使用：`{}` 与 `{}`", first.flag(), second.flag())
            }
            Self::MissingCommand => write!(f, "`--json` 需要搭配一个命令"),
        }
    }
}

/// 解析参数（`args` 不含程序名）
///
/// - `Ok(None)`：没有任何 CLI 意图 ⇒ 正常启动图形界面
/// - `Ok(Some(..))`：执行该调用后退出
/// - `Err(..)`：用法错误
pub fn parse<I>(args: I) -> Result<Option<Invocation>, UsageError>
where
    I: IntoIterator<Item = String>,
{
    let mut command: Option<Command> = None;
    let mut format = Format::Human;

    for arg in args {
        if let Some(parsed) = Command::from_flag(&arg) {
            // 帮助优先：带了帮助就只打帮助，不再校验其它命令
            if parsed == Command::Help {
                return Ok(Some(Invocation { command: parsed, format }));
            }
            if let Some(previous) = command {
                return Err(UsageError::Conflict(previous, parsed));
            }
            command = Some(parsed);
            continue;
        }

        if arg == "--json" {
            format = Format::Json;
            continue;
        }

        if arg.starts_with('-') {
            return Err(UsageError::Unknown(arg));
        }

        // 位置参数：忽略（见模块文档：更新重启会转发原启动参数）
    }

    match command {
        Some(command) => Ok(Some(Invocation { command, format })),
        None if format == Format::Json => Err(UsageError::MissingCommand),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(args: &[&str]) -> Result<Option<Invocation>, UsageError> {
        parse(args.iter().map(|s| s.to_string()))
    }

    fn invocation(command: Command, format: Format) -> Option<Invocation> {
        Some(Invocation { command, format })
    }

    #[test]
    fn no_args_means_launch_gui() {
        assert_eq!(parse_str(&[]), Ok(None));
    }

    #[test]
    fn positional_args_are_ignored() {
        // 更新重启会带回原启动参数：不认识的「非 - 开头」参数不能把应用拦下来
        assert_eq!(parse_str(&["some-file.txt"]), Ok(None));
        assert_eq!(
            parse_str(&["--mode", "some-file.txt"]),
            Ok(invocation(Command::Mode, Format::Human))
        );
    }

    #[test]
    fn recognizes_single_command() {
        assert_eq!(parse_str(&["-v"]), Ok(invocation(Command::Version, Format::Human)));
        assert_eq!(parse_str(&["--version"]), Ok(invocation(Command::Version, Format::Human)));
        assert_eq!(
            parse_str(&["--build-info"]),
            Ok(invocation(Command::BuildInfo, Format::Human))
        );
        assert_eq!(parse_str(&["--doctor"]), Ok(invocation(Command::Doctor, Format::Human)));
    }

    #[test]
    fn json_is_a_modifier_in_any_position() {
        assert_eq!(
            parse_str(&["--json", "--build-info"]),
            Ok(invocation(Command::BuildInfo, Format::Json))
        );
        assert_eq!(
            parse_str(&["--build-info", "--json"]),
            Ok(invocation(Command::BuildInfo, Format::Json))
        );
        // 重复出现无害
        assert_eq!(
            parse_str(&["--json", "--app-paths", "--json"]),
            Ok(invocation(Command::AppPaths, Format::Json))
        );
    }

    #[test]
    fn conflicting_commands_are_rejected() {
        assert_eq!(
            parse_str(&["--version", "--app-paths"]),
            Err(UsageError::Conflict(Command::Version, Command::AppPaths))
        );
    }

    #[test]
    fn unknown_flags_are_rejected() {
        // 老实现的坑：拼错会静默启动 GUI
        assert_eq!(parse_str(&["--versionn"]), Err(UsageError::Unknown("--versionn".into())));
        assert_eq!(parse_str(&["-x"]), Err(UsageError::Unknown("-x".into())));
    }

    #[test]
    fn json_alone_is_usage_error() {
        assert_eq!(parse_str(&["--json"]), Err(UsageError::MissingCommand));
    }

    #[test]
    fn help_wins_over_other_commands() {
        assert_eq!(parse_str(&["-v", "--help"]), Ok(invocation(Command::Help, Format::Human)));
        assert_eq!(parse_str(&["-h", "--nonsense"]), Ok(invocation(Command::Help, Format::Human)));
    }
}
