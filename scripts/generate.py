#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
手动执行代码生成入口

用法:
    uv run ./scripts/generate.py
"""

import sys

from codegen import CodegenError, generate_all
from common.logger import error
from util.sigint import graceful_sigint

if __name__ == "__main__":
    with graceful_sigint():
        try:
            generate_all()
        except CodegenError as e:
            error(f"代码生成失败: {e}")
            sys.exit(1)
