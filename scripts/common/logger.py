#!/usr/bin/env python3
# -*- coding: UTF-8 -*-
# @Author  : Serene Lee
# @Date    : 2026/6/6

"""
公共日志模块。

一、日志记录
    底层入口为 log(level, message)，按级别封装为：
        debug(msg) / info(msg) / warning(msg) / error(msg)

    每条日志同时送往两个 handler：
      - 控制台：按级别带 ANSI 颜色，供开发者阅读。
      - 文件：无颜色、纯文本，路径为 <项目根>/data/logs/p/<YYYY-MM-DD>.log，
        UTF-8 编码，按当天日期归档，可放心 grep 与解析。

    两侧行格式一致：
        [<UTC 时间>] [<级别>] [<文件名>:<行号>] <消息>
    时间为 UTC、ISO 8601、毫秒精度、以 Z 结尾，不随机器时区漂移。
    两个 handler 及 root logger 的级别均为 INFO。

二、密钥打码
    redact(message) 抹掉消息中的疑似凭据。log() 在输出与落盘前对每条消息调用它。

    依次应用四条规则：
      1. 已知密钥环境变量（_SECRET_ENV_VARS）的值原样出现 ->
        替换为 `<变量名>=***`；值短于 _MIN_SECRET_LEN 的不处理，避免误伤短词。
      2. URL 内嵌凭据 scheme://user:password@host ->
        密码段替换为 `***`。
      3. 查询串中名字含 `token` / `key` / `secret` / `password`/ `signature` / `credential` / `sig` 的参数 ->
        只保留参数名，值替换为 `***`。
      4. 形如 *TOKEN=* / *PASSWORD=* 等赋值样式的值 ->
        替换为 `***`。

    注意：redact() 是"尽力而为"，不是"证明安全"。
    新增密钥传递方式时，应同步扩充 _SECRET_ENV_VARS 或补充相应正则。

三、用法
    from common.logger import info, warning, error, debug

    info("开始构建")
    error(f"下载失败：{url}")   # url 中若含 token，会被自动打码

    首次调用任一函数时完成 logging 配置，之后复用；日志目录不存在会自动创建。
"""
import logging
import os
import re
import sys
from datetime import date, datetime, timezone

from common.constant.paths import PROJECT_BASE_DIR

# 这些环境变量的值一旦出现在日志里就按密钥处理，统一打码
# （约定：密钥只经环境变量传递，绝不进命令行参数——argv 会被命令级日志原样打印）
_SECRET_ENV_VARS = (
    "CNB_TOKEN",
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "MIMO_TTS_API_KEY",
    "TAURI_SIGNING_PRIVATE_KEY",
    "TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
)
# URL 内嵌凭据（如 https://user:token@host）中的密码段
_URL_USERINFO_RE = re.compile(r"(?<=://)([^/@:\s]+):([^@\s/]+)@")
# 查询串中名字带敏感词的参数值（如 ?signature=...&upload_token=...）
_SENSITIVE_QUERY_RE = re.compile(
    r"([?&][^&\s=]*(?:token|key|secret|password|signature|credential|sig)[^&\s=]*=)[^&\s]+",
    re.IGNORECASE,
)
# 通用兜底：形如 <含 TOKEN/KEY/SECRET/PASSWORD/CREDENTIAL 的变量名>=值 或 ": 值" 的赋值样式，
# 值一律打码——防的是未列入 _SECRET_ENV_VARS 的密钥变量被人打印
_ASSIGNMENT_RE = re.compile(
    r"([A-Za-z0-9_]*(?:token|key|secret|password|credential)[A-Za-z0-9_]*\s*[=:]\s*)(\S+)",
    re.IGNORECASE,
)
# 参与打码的环境变量值最小长度：短于它不替换，避免误伤普通短词
_MIN_SECRET_LEN = 8

# 日志行格式：文件与控制台共用，保证两侧内容一致，只有颜色不同
_LOG_FORMAT = "[%(asctime)s] %(levelname_padded)s [%(filename)s:%(lineno)d] %(message)s"

# 全局标志，防止重复配置
_LOGGING_CONFIGURED = False

SUCCESS = 25
logging.addLevelName(SUCCESS, "SUCCESS")


def _fg(r: int, g: int, b: int) -> str:
    """前景色（文字色）转义序列。"""
    return f"\033[38;2;{r};{g};{b}m"


def _bg_fg(bg_r: int, bg_g: int, bg_b: int, fg_r: int, fg_g: int, fg_b: int) -> str:
    """背景色 + 前景色 转义序列。"""
    return f"\033[48;2;{bg_r};{bg_g};{bg_b};38;2;{fg_r};{fg_g};{fg_b}m"


def redact(message: str) -> str:
    """抹掉消息里的疑似密钥，返回安全文本。

    三条规则：
    1. 已知密钥环境变量（_SECRET_ENV_VARS）的值原样出现 → 替换为 `<变量名>=***`；
       值短于 _MIN_SECRET_LEN 的不处理，避免把普通短词误伤。
    2. URL 内嵌凭据 `scheme://user:password@host` → 密码段替换为 ***。
    3. 查询串中名字含 token/key/secret/password/signature/credential/sig 的参数
       → 只留参数名，值替换为 ***。
    4. 兜底：形如 `*TOKEN=*` / `*PASSWORD=*` 等赋值样式的值 → 替换为 ***。
    """
    for name in _SECRET_ENV_VARS:
        value = os.environ.get(name, "")
        if len(value) >= _MIN_SECRET_LEN:
            message = message.replace(value, f"{name}=***")
    message = _URL_USERINFO_RE.sub(r"\1:***@", message)
    message = _SENSITIVE_QUERY_RE.sub(r"\1***", message)
    message = _ASSIGNMENT_RE.sub(r"\1***", message)
    return message


def use_color() -> bool:
    """决定控制台 handler 是否输出 ANSI 颜色。

    优先级：显式开关 > 环境能力 > 兜底关闭。
    """
    if os.environ.get("NO_COLOR"):  # 社区约定：存在且非空即强制关闭
        return False
    if os.environ.get("FORCE_COLOR"):  # 社区约定：存在即强制开启
        return True
    if os.environ.get("TERM") == "dumb":
        return False
    # 非 tty（重定向、管道、CI 抓取）一律不上色
    return hasattr(sys.stdout, "isatty") and sys.stdout.isatty()


# 自定义 Formatter，输出 UTC 时间；文件 handler 使用，不带任何颜色转义
class FileFormatter(logging.Formatter):
    LEVEL_WIDTH = 10

    def formatTime(self, record, datefmt=None):
        # record.created 是 UTC 时间戳
        dt = datetime.fromtimestamp(record.created, tz=timezone.utc)
        if datefmt:
            return dt.strftime(datefmt)
        return dt.strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"

    def format(self, record):
        # 提前构造好带中括号、且右补空格的级别标签，如 "[DEBUG]   "。
        # 空格补在 "]" 之后，保证中括号紧贴标签，字体连写不被撑开。
        # 注入的是派生字段，不覆盖 record.levelname 本身，对上层无副作用。
        record.levelname_padded = f"[{record.levelname}]".ljust(self.LEVEL_WIDTH)
        return super().format(record)


# 控制台专用：在 FileFormatter 的输出上按日志级别包一层 ANSI 颜色。
# 除颜色外，时间戳格式、字段顺序、异常栈等都完全继承自 FileFormatter。
class ConsoleFormatter(FileFormatter):
    COLORS = {
        logging.DEBUG: _fg(128, 128, 128),  # 中灰,
        logging.INFO: _fg(80, 160, 255),  # 亮蓝,
        logging.WARNING: _fg(255, 200, 0),  # 琥珀黄,
        logging.ERROR: _fg(255, 80, 80),  # 亮红,
        logging.CRITICAL: _bg_fg(200, 0, 0, 255, 255, 255),  # 红底白字
        SUCCESS: _fg(80, 255, 120)  # 绿色
    }
    RESET = "\033[0m"

    def format(self, record):
        s = super().format(record)
        color = self.COLORS.get(record.levelno, "")
        # 未知级别 color 为空串，此时不加 RESET，避免往输出里塞无意义的转义
        return f"{color}{s}{self.RESET}" if color else s


def _setup_logging():
    global _LOGGING_CONFIGURED
    if _LOGGING_CONFIGURED:
        return

    log_dir = PROJECT_BASE_DIR / "data" / "logs" / "p"
    log_dir.mkdir(parents=True, exist_ok=True)

    file_handler = logging.FileHandler(log_dir / f"{date.today()}.log", encoding="utf-8")
    file_handler.setLevel(logging.INFO)
    file_handler.setFormatter(FileFormatter(_LOG_FORMAT))

    console_handler = logging.StreamHandler(sys.stdout)
    console_handler.setLevel(logging.DEBUG)
    if use_color():
        console_handler.setFormatter(ConsoleFormatter(_LOG_FORMAT))
    else:
        console_handler.setFormatter(FileFormatter(_LOG_FORMAT))

    root_logger = logging.getLogger()
    root_logger.addHandler(console_handler)
    root_logger.addHandler(file_handler)
    root_logger.setLevel(logging.DEBUG)

    _LOGGING_CONFIGURED = True


def log(level: str, message: str) -> None:
    _setup_logging()

    level_upper = level.upper()
    level_value = getattr(logging, level_upper, logging.INFO)

    # 输出前统一脱敏：密钥环境变量值 / URL 内嵌凭据 / 敏感查询参数
    message = redact(str(message))

    # stacklevel=2 让 logging 记录调用 log() 的调用者的位置
    logging.log(level_value, message, stacklevel=2)


def debug(message: str) -> None:
    log("DEBUG", message)


def info(message: str) -> None:
    log("INFO", message)


def warning(message: str) -> None:
    log("WARNING", message)


def error(message: str) -> None:
    log("ERROR", message)


def critical(message: str) -> None:
    log("CRITICAL", message)


def success(message: str) -> None:
    _setup_logging()
    message = redact(str(message))
    logging.log(SUCCESS, message, stacklevel=2)
