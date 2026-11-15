#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
node 命令封装
"""

from cli.tool import Timeout, Tool
from common.constant.paths import PROJECT_BASE_DIR


class NodeError(Exception):
    """node 命令无法启动或执行失败"""


TOOL = Tool(
    "node",
    NodeError,
    cwd=PROJECT_BASE_DIR,
    timeout=Timeout.QUERY,
    hint="请安装 Node.js 并加入 PATH：https://nodejs.org/",
)
