#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
just 命令封装
"""

import subprocess
from pathlib import Path

from cli import _process
from cli._version import version_string
from common.constant.paths import PROJECT_BASE_DIR

# 单条 just 命令的最大超时时长（秒）
RUN_TIMEOUT = 300


class JustError(Exception):
    """just 命令无法启动或执行失败"""


def _run(args: list[str], *, cwd: Path | None = None, timeout: float | int = RUN_TIMEOUT,
         capture: bool = True) -> subprocess.CompletedProcess:
    """执行 just 命令，失败统一包成 JustError"""
    cwd = cwd or PROJECT_BASE_DIR
    try:
        return _process.run(["just", *args], cwd=cwd, timeout=timeout, capture=capture)
    except _process.ProcessError as error:
        if error.not_found:
            raise JustError("未找到 'just'，请安装后加入 PATH") from error
        raise JustError(str(error)) from error


def version() -> str | None:
    """`just --version` 里的版本号；输出认不出时返回 None"""
    return version_string(str(_run(["--version"]).stdout))
