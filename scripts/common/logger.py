#!/usr/bin/env python3
# -*- coding: UTF-8 -*-
# @Author  : Serene Lee
# @Date    : 2026/6/6

"""
Common utilities for JRE build and dependency analysis scripts. \n

1. log recording \n
   - log()：Print a timestamped log message in the format: [<epoch_seconds>] [LEVEL] message.

2. secret redaction \n
   - redact()：Scrub suspected secrets from a message（已知密钥环境变量的值、URL 内嵌
     凭据、敏感查询参数）。log() 输出的每一行都会先过它——密钥统一走环境变量的前提下，
     这是"谁忘了小心也不会泄漏"的最后一道闸门。

3. convenient tools \n
   - find_exec()：Locate an executable in PATH.
   - is_windows()：Check if it is currently running on a Windows system.
"""

import logging
import os
import re
import sys
from datetime import date, datetime, timezone
from pathlib import Path

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

# 全局标志，防止重复配置
_LOGGING_CONFIGURED = False


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


def _setup_logging():
    global _LOGGING_CONFIGURED
    if _LOGGING_CONFIGURED:
        return

    # 自定义 Formatter，输出 UTC 时间
    class UTCFormatter(logging.Formatter):
        def formatTime(self, record, datefmt=None):
            # record.created 是 UTC 时间戳
            dt = datetime.fromtimestamp(record.created, tz=timezone.utc)
            if datefmt:
                return dt.strftime(datefmt)
            return dt.strftime("%Y-%m-%d %H:%M:%S.%f")[:-3] + " UTC"

    formatter = UTCFormatter("[%(asctime)s] [%(levelname)s] [%(filename)s:%(lineno)d] %(message)s")

    # 日志目录固定为项目根目录/data/logs/p（基于本文件位置推导，避免依赖 cwd）
    # logger.py 位于 scripts/common/ → parents[2] 即项目根目录
    log_dir = Path(__file__).resolve().parents[2] / "data" / "logs" / "p"
    log_dir.mkdir(parents=True, exist_ok=True)

    file_handler = logging.FileHandler(log_dir / f"{date.today()}.log", encoding="utf-8")
    # 排查 CI 需要完整流程，文件记录 INFO 及以上（与控制台一致）
    file_handler.setLevel(logging.INFO)
    file_handler.setFormatter(formatter)

    console_handler = logging.StreamHandler(sys.stdout)
    console_handler.setLevel(logging.INFO)
    console_handler.setFormatter(formatter)

    root_logger = logging.getLogger()
    root_logger.addHandler(console_handler)
    root_logger.addHandler(file_handler)
    root_logger.setLevel(logging.INFO)

    _LOGGING_CONFIGURED = True


def log(level: str, message: str) -> None:
    _setup_logging()

    level_upper = level.upper()
    level_value = getattr(logging, level_upper, logging.INFO)

    # 输出前统一脱敏：密钥环境变量值 / URL 内嵌凭据 / 敏感查询参数
    message = redact(str(message))

    # stacklevel=2 让 logging 记录调用 log() 的调用者的位置
    logging.log(level_value, message, stacklevel=2)
