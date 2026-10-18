#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
uv 命令封装
"""

from pathlib import Path

from cli import process
from common.constant.paths import PROJECT_BASE_DIR
from common.logger import warning

# 首次解析全部依赖可能较慢，给足时间
LOCK_TIMEOUT = 300


def lock(cwd: Path | None = None) -> None:
    """执行 `uv lock` 刷新 uv.lock，失败只警告，锁文件滞后不该阻断构建"""
    try:
        process.run(["uv", "lock"], cwd=cwd or PROJECT_BASE_DIR, timeout=LOCK_TIMEOUT)
    except process.ProcessError as error:
        warning(f"uv lock 未完成，uv.lock 可能滞后: {error}")
