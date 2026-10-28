#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
从工具的 `--version` 输出里取出裸版本号
"""

import re

# 输出里的第一段数字，最多三段
# `git version 2.55.0.windows.3`、`v22.22.2`、`cargo 1.96.0 (...)` 都能处理
_VERSION_RE = re.compile(r"(\d+(?:\.\d+){0,2})")


def version_string(output: str) -> str | None:
    """取输出里第一段数字，否则返回 None"""
    match = _VERSION_RE.search(output)
    return match.group(1) if match else None
