#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
node 命令封装
"""

from cli._tool import Tool
from common.constant.paths import PROJECT_BASE_DIR

# 单条 node 命令的最大超时时长（秒）
RUN_TIMEOUT = 60


class NodeError(Exception):
    """node 命令无法启动或执行失败"""


TOOL = Tool(
    "node",
    NodeError,
    cwd=PROJECT_BASE_DIR,
    timeout=RUN_TIMEOUT,
    hint="请安装 Node.js 并加入 PATH：https://nodejs.org/",
)
