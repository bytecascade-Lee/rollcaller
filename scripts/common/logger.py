#!/usr/bin/env python3
# -*- coding: UTF-8 -*-
# @Author  : Serene Lee
# @Date    : 2026/6/6

"""
Common utilities for JRE build and dependency analysis scripts. \n

1. log recording \n
   - log()：Print a timestamped log message in the format: [<epoch_seconds>] [LEVEL] message.

2. convenient tools \n
   - find_exec()：Locate an executable in PATH.
   - is_windows()：Check if it is currently running on a Windows system.
"""

import logging
import sys
from datetime import date, datetime, timezone
from pathlib import Path

# 全局标志，防止重复配置
_LOGGING_CONFIGURED = False


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

    # stacklevel=2 让 logging 记录调用 log() 的调用者的位置
    logging.log(level_value, message, stacklevel=2)
