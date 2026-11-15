#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
ruff 命令封装

ruff 随 dev 组安装进项目虚拟环境，不在 PATH 上：从当前解释器同目录取可执行文件，
保证「谁在跑脚本，就用谁的那一份工具」，与 uv.lock 锁定的版本严格一致。
"""

import subprocess
import sys
from pathlib import Path

from cli import _process
from common.constant.paths import PROJECT_BASE_DIR

RUN_TIMEOUT = 120


class RuffError(Exception):
    """ruff 无法启动，或检查发现违规"""


def _run(args: list[str], *, cwd: Path | None = None, capture: bool = False) -> subprocess.CompletedProcess[str]:
    """执行 ruff 命令，失败统一包成 RuffError；诊断默认进控制台"""
    cwd = cwd or PROJECT_BASE_DIR
    try:
        program = _process.resolve("ruff", search_dir=Path(sys.executable).parent)
        return _process.run([program, *args], cwd=cwd, timeout=RUN_TIMEOUT, capture=capture)
    except _process.ProcessError as error:
        if error.not_found:
            raise RuffError("未在当前虚拟环境找到 'ruff'，请执行 `just setup` 同步依赖") from error
        raise RuffError(str(error)) from error


def check(*, cwd: Path | None = None) -> None:
    """按照 pyproject.toml 中的 [tool.ruff] 执行 lint 检查"""
    _ = _run(["check"], cwd=cwd)


def format_check(*, cwd: Path | None = None) -> None:
    """按照 pyproject.toml 中的 [tool.ruff] 检查格式，不写文件"""
    _ = _run(["format", "--check"], cwd=cwd)
