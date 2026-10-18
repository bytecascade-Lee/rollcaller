#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
构建矩阵

来源唯一：`common.support.pipeline_entries()`（同时开放 BUILD 与 RELEASE 的目标）。
矩阵与产物发现共用这一份集合，避免 workflow 手写列表与支持表漂移。
"""

import json

from common import support


class MatrixError(Exception):
    """构建矩阵生成失败"""


def build_matrix() -> list[dict[str, str]]:
    """每个条目包含 target / runner / arch，供 matrix 与 runs-on 直接使用"""
    entries = support.pipeline_entries()
    if not entries:
        raise MatrixError("支持表中没有同时开放 BUILD 与 RELEASE 的目标")

    return [
        {
            "target": entry.target.triple,
            "runner": entry.runner,
            "arch": entry.target.arch.value,
        }
        for entry in entries
    ]


def render(matrix: list[dict[str, str]]) -> str:
    """单行 JSON，可直接写进 GITHUB_OUTPUT"""
    return json.dumps(matrix, ensure_ascii=False, separators=(",", ":"))
