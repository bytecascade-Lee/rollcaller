#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
just 命令封装
"""

from cli._tool import Tool
from common.constant.paths import PROJECT_BASE_DIR

# 单条 just 命令的最大超时时长（秒）
RUN_TIMEOUT = 300


class JustError(Exception):
    """just 命令无法启动或执行失败"""


TOOL = Tool(
    "just",
    JustError,
    cwd=PROJECT_BASE_DIR,
    timeout=RUN_TIMEOUT,
    hint="请安装 just 并加入 PATH：https://just.systems/man/en/installation.html",
)
