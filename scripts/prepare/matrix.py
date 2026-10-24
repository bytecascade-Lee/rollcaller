#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
构建矩阵
"""

import json

from common import support


class MatrixError(Exception):
    """构建矩阵生成失败"""


def build_matrix() -> str:
    """每个条目包含 target / os / runner / arch，供 matrix、条件与 runs-on 直接使用，返回渲染为 1 行的 json 字符串"""
    entries = support.pipeline_entries()
    if not entries:
        raise MatrixError("支持表中没有同时开放 BUILD 与 RELEASE 的目标")

    matrix = [
        {
            "target": entry.target.triple,
            "runner": entry.runner,
            "os": entry.target.os.value,
            "arch": entry.target.arch.value,
        }
        for entry in entries
    ]
    return json.dumps(matrix, ensure_ascii=False, separators=(",", ":"))
