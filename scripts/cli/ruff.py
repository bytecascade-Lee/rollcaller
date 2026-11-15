#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
ruff 命令封装

ruff 随 dev 组安装进项目虚拟环境，不在 PATH 上：从当前解释器同目录取可执行文件，
保证「谁在跑脚本，就用谁的那一份工具」，与 uv.lock 锁定的版本严格一致。
"""

import sys
from pathlib import Path

from cli.tool import Timeout, Tool
from common.constant.paths import PROJECT_BASE_DIR


class RuffError(Exception):
    """ruff 无法启动，或检查发现违规"""


TOOL = Tool(
    "ruff",
    RuffError,
    cwd=PROJECT_BASE_DIR,
    timeout=Timeout.NORMAL,
    capture=False,
    search_dir=Path(sys.executable).parent,
    hint="请执行 `just setup` 同步依赖（ruff 随 dev 组安装）",
)


def lint() -> None:
    """按照 pyproject.toml 中的 [tool.ruff] 执行 lint 检查"""
    _ = TOOL.run(["check"])


def format(*, check: bool = False) -> None:
    """
    按照 pyproject.toml 中的 [tool.ruff] 格式化

    :param check: True 时只检查不写文件
    """
    args = ["format", *(["--check"] if check else [])]
    _ = TOOL.run(args)
