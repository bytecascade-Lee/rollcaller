#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
日志模块

- 入口：debug / info / success / warning / error，其中success 是自定义级别 25
- 行格式：``<UTC 时间> [级别] [文件:行号] 消息``，两条通道完全一致，只差颜色
- 文件：``data/logs/python/<YYYY-MM>/<YYYY-MM-DD>.log``，UTF-8、无颜色，按天归档，可直接 rg
- 级别：root 与控制台 ``DEBUG``，文件 ``INFO``
- 所有消息先过 redact() 打码，规则只在该函数里维护
"""

import logging
import os
import re
import sys
from datetime import date, datetime, timezone
from functools import cache
from pathlib import Path

from common.constant.paths import DEVELOP_DATA_DIR
from util.fs import relative_path
from util.time import format_to_iso_8601

SUCCESS = 25
logging.addLevelName(SUCCESS, "SUCCESS")

# 此处环境变量的值一旦出现在日志里就按密钥处理，统一打码
_SECRET_ENV_VARS = (
    "CNB_TOKEN",
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "MIMO_TTS_API_KEY",
    "TAURI_SIGNING_PRIVATE_KEY",
    "TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
)
# URL 内嵌凭据中的密码段，如 https://user:token@host
_URL_USERINFO_RE = re.compile(r"(?<=://)([^/@:\s]+):([^@\s/]+)@")
# 查询串中名字带敏感词的参数值，如 ?signature=...&upload_token=...
_SENSITIVE_QUERY_RE = re.compile(
    r"([?&][^&\s=]*(?:token|key|secret|password|signature|credential|sig)[^&\s=]*=)[^&\s]+",
    re.IGNORECASE
)
# 形如 <含 TOKEN/KEY/SECRET/PASSWORD/CREDENTIAL 的变量名> 值 或 ": 值" 的赋值样式，值一律打码
_ASSIGNMENT_RE = re.compile(
    r"([A-Za-z0-9_]*(?:token|key|secret|password|credential)[A-Za-z0-9_]*\s*[=:]\s*)(\S+)",
    re.IGNORECASE,
)
# 参与打码的环境变量值最小长度
_MIN_SECRET_LEN = 8


def redact(message: str) -> str:
    """
    抹掉消息里的疑似密钥，返回安全文本。

    1. 已知密钥环境变量的值原样出现 -> 替换为 `<变量名>=***`，值短于 _MIN_SECRET_LEN 的不处理
    2. URL 内嵌凭据 `scheme://user:password@host` -> 密码段替换为 ***
    3. 查询串中名字含 token/key/secret/password/signature/credential/sig 的参数 -> 只留参数名，值替换为 ***
    4. 形如 `*TOKEN=*` / `*PASSWORD=*` 等赋值样式的值 -> 替换为 ***。

    此函数只保证 「尽力而为」
    """
    for name in _SECRET_ENV_VARS:
        value = os.environ.get(name, "")
        if len(value) >= _MIN_SECRET_LEN:
            message = message.replace(value, f"{name}=***")
    message = _URL_USERINFO_RE.sub(r"\1:***@", message)
    message = _SENSITIVE_QUERY_RE.sub(r"\1***", message)
    message = _ASSIGNMENT_RE.sub(r"\1***", message)
    return message


def _fg(r: int, g: int, b: int) -> str:
    """前景色转义序列"""
    return f"\033[38;2;{r};{g};{b}m"


def _bg_fg(bg_r: int, bg_g: int, bg_b: int, fg_r: int, fg_g: int, fg_b: int) -> str:
    """背景色 + 前景色转义序列"""
    return f"\033[48;2;{bg_r};{bg_g};{bg_b};38;2;{fg_r};{fg_g};{fg_b}m"


_RESET = "\033[0m"
_DIM = _fg(128, 128, 128)  # 时间戳、位置，以及 DEBUG 标签
_BODY = _fg(180, 180, 180)  # 正文颜色
_LEVEL_COLORS = {
    logging.DEBUG: _DIM,
    logging.INFO: _fg(80, 160, 255),
    logging.WARNING: _fg(255, 200, 0),
}
_SUCCESS_STYLE = _fg(80, 255, 120)  # SUCCESS 整行绿字
_ERROR_STYLE = _bg_fg(200, 0, 0, 255, 255, 255)  # ERROR 整行红底白字

TAG_WIDTH = 10  # 级别标签补齐宽度


def use_color() -> bool:
    """控制台是否输出 ANSI 颜色：显式开关 > 终端能力 > 兜底关闭"""
    if os.environ.get("NO_COLOR"):  # 社区约定：存在且非空即强制关闭
        return False
    if os.environ.get("FORCE_COLOR", "") not in ("", "0"):  # FORCE_COLOR=0 视为关闭
        return True
    if os.environ.get("TERM") == "dumb":
        return False
    # 非 tty（重定向、管道、CI 抓取）一律不上色
    return sys.stdout.isatty()


class LogFormatter(logging.Formatter):
    """把一条记录渲染成一行"""

    def __init__(self, *, color: bool) -> None:
        super().__init__()
        self.color = color

    def formatTime(self, record, datefmt: str | None = None) -> str:
        """UTC、ISO 8601、毫秒、以 Z 结尾，不随机器时区漂移"""
        return format_to_iso_8601(datetime.fromtimestamp(record.created))

    def format(self, record: logging.LogRecord) -> str:
        stamp = self.formatTime(record)
        tag = f"[{record.levelname}]".ljust(TAG_WIDTH)
        location = f"[{relative_path(record.pathname)}:{record.lineno}]"
        message = record.getMessage()

        if record.exc_info and not record.exc_text:
            record.exc_text = self.formatException(record.exc_info)
        if record.exc_text:
            message = f"{message}\n{record.exc_text}"

        if not self.color:
            return f"{stamp} {tag} {location} {message}"
        if record.levelno == logging.ERROR:
            return f"{_ERROR_STYLE}{stamp} {tag} {location} {message}{_RESET}"
        if record.levelno == SUCCESS:
            return f"{_SUCCESS_STYLE}{stamp} {tag} {location} {message}{_RESET}"

        tag_color = _LEVEL_COLORS.get(record.levelno)
        level = f"{tag_color}{tag}{_RESET}" if tag_color else tag
        return f"{_DIM}{stamp}{_RESET} {level} {_DIM}{location}{_RESET} {_BODY}{message}{_RESET}"


class DailyFileHandler(logging.FileHandler):
    """按记录时间跨天 / 跨月自动切文件"""

    def __init__(self, log_root: Path, encoding: str = "UTF-8") -> None:
        self._log_root = log_root
        self._current_date = datetime.now(tz=timezone.utc).date()
        super().__init__(self._path_for(self._current_date), encoding=encoding, delay=True)

    def _path_for(self, d: date) -> Path:
        return self._log_root / f"{d.year}-{d.month:02}" / f"{d.isoformat()}.log"

    def emit(self, record: logging.LogRecord) -> None:
        # 用记录自身的 created 时间，保证异步/队列场景正确
        try:
            d = datetime.fromtimestamp(record.created, tz=timezone.utc).date()
            if d != self._current_date:
                self._current_date = d
                self.close()
                path = self._path_for(d)
                path.parent.mkdir(parents=True, exist_ok=True)
                self.baseFilename = os.fspath(path)
        except Exception:
            self.handleError(record)
            return
        super().emit(record)


@cache
def _setup() -> None:
    """首次记录日志时装配 handler"""
    console_handler = logging.StreamHandler(sys.stdout)
    console_handler.setLevel(logging.DEBUG)
    console_handler.setFormatter(LogFormatter(color=use_color()))

    file_handler = DailyFileHandler(DEVELOP_DATA_DIR / "logs" / "python")
    file_handler.setLevel(logging.INFO)
    file_handler.setFormatter(LogFormatter(color=False))

    root = logging.getLogger()
    root.setLevel(logging.DEBUG)
    root.addHandler(console_handler)
    root.addHandler(file_handler)


def _emit(level: int, message: str, stacklevel: int) -> None:
    """统一落点：装配、脱敏、定位调用方"""
    _setup()
    logging.log(level, redact(str(message)), stacklevel=stacklevel + 2)


def debug(message: str, *, stacklevel: int = 1) -> None:
    _emit(logging.DEBUG, message, stacklevel)


def info(message: str, *, stacklevel: int = 1) -> None:
    _emit(logging.INFO, message, stacklevel)


def success(message: str, *, stacklevel: int = 1) -> None:
    _emit(SUCCESS, message, stacklevel)


def warning(message: str, *, stacklevel: int = 1) -> None:
    _emit(logging.WARNING, message, stacklevel)


def error(message: str, *, stacklevel: int = 1) -> None:
    _emit(logging.ERROR, message, stacklevel)
