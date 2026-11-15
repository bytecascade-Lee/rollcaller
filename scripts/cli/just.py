#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
just 命令封装
"""

from cli._tool import Timeout, Tool
from common.constant.paths import PROJECT_BASE_DIR


class JustError(Exception):
    """just 命令无法启动或执行失败"""


TOOL = Tool(
    "just",
    JustError,
    cwd=PROJECT_BASE_DIR,
    timeout=Timeout.NORMAL,
    hint="请安装 just 并加入 PATH：https://just.systems/man/en/installation.html",
)
