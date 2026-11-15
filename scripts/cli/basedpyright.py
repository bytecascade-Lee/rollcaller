#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
basedpyright 命令封装
"""

import sys
from pathlib import Path

from cli.tool import Timeout, Tool
from common.constant.paths import PROJECT_BASE_DIR


class BasedpyrightError(Exception):
    """basedpyright 无法启动，或检查发现类型错误"""


TOOL = Tool(
    "basedpyright",
    BasedpyrightError,
    cwd=PROJECT_BASE_DIR,
    timeout=Timeout.NORMAL,
    capture=False,
    search_dir=Path(sys.executable).parent,
    hint="请执行 `just setup` 同步依赖（basedpyright 随 dev 组依赖安装）",
)


def typecheck() -> None:
    """按照 pyproject.toml 中的 [tool.basedpyright] 执行类型检查"""
    _ = TOOL.run(["--level", "error"])
