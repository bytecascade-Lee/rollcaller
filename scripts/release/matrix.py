#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
生成 CI 构建 + 发布矩阵，来源于 `common.support.pipeline_entries()`

输出：
    - 设置 GITHUB_OUTPUT 时：写 `include=<单行 JSON>`，不往 stdout 打任何内容
    - 否则：把单行 JSON 打到 stdout
"""

import json
import os
import sys

from common import support


def build_matrix() -> list[dict[str, str]]:
    """每个条目包含 target / runner / arch，供 matrix 与 runs-on 直接使用"""
    return [
        {
            "target": entry.target.triple,
            "runner": entry.runner,
            "arch": entry.target.arch.value,
        }
        for entry in support.pipeline_entries()
    ]


def main() -> int:
    matrix = build_matrix()
    if not matrix:
        print("支持表中没有同时开放 BUILD 与 RELEASE 的目标", file=sys.stderr)
        return 1

    payload = json.dumps(matrix, ensure_ascii=False, separators=(",", ":"))
    github_output = os.environ.get("GITHUB_OUTPUT")
    if github_output:
        with open(github_output, "a", encoding="utf-8") as file:
            file.write(f"include={payload}\n")
    else:
        print(payload)
    return 0


if __name__ == "__main__":
    sys.exit(main())
