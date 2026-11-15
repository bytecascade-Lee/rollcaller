#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
basedpyright 命令封装
"""

import subprocess
import sys
from pathlib import Path

from cli import _process
from common.constant.paths import PROJECT_BASE_DIR

# 冷启动要拉起内嵌的 node
RUN_TIMEOUT = 300


class BasedpyrightError(Exception):
    """basedpyright 无法启动，或检查发现类型错误"""


def _run(args: list[str], *, cwd: Path | None = None, capture: bool = False) -> subprocess.CompletedProcess[str]:
    """执行 basedpyright 命令，失败统一包成 BasedpyrightError；报告默认直接进控制台"""
    cwd = cwd or PROJECT_BASE_DIR
    try:
        program = _process.resolve("basedpyright", search_dir=Path(sys.executable).parent)
        return _process.run([program, *args], cwd=cwd, timeout=RUN_TIMEOUT, capture=capture)
    except _process.ProcessError as error:
        if error.not_found:
            raise BasedpyrightError("未在当前虚拟环境找到 'basedpyright'，请执行 `just setup` 同步依赖") from error
        raise BasedpyrightError(str(error)) from error


def check(*, cwd: Path | None = None) -> None:
    """按照 pyproject.toml 中的 [tool.basedpyright] 执行类型检查，等级为 `error`"""
    _ = _run(["--level", "error"], cwd=cwd)
