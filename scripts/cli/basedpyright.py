#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
basedpyright 命令封装

与 ruff 一样随 dev 组安装进项目虚拟环境：从当前解释器同目录取可执行文件，
不依赖 PATH，也不受 IDE 自带的语言服务器影响。
"""

import sys
from pathlib import Path

from cli._tool import Tool
from common.constant.paths import PROJECT_BASE_DIR

# 冷启动要拉起内嵌的 node
RUN_TIMEOUT = 300


class BasedpyrightError(Exception):
    """basedpyright 无法启动，或检查发现类型错误"""


TOOL = Tool(
    "basedpyright",
    BasedpyrightError,
    cwd=PROJECT_BASE_DIR,
    timeout=RUN_TIMEOUT,
    capture=False,
    search_dir=Path(sys.executable).parent,
    hint="请执行 `just setup` 同步依赖（basedpyright 随 dev 组依赖安装）",
)


def typecheck() -> None:
    """按照 pyproject.toml 中的 [tool.basedpyright] 执行类型检查"""
    _ = TOOL.run(["--level", "error"])
