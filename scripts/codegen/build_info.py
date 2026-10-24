#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
生成 backend/gen/build/build_info.json，构建元数据的唯一载体

调用时机：
    开发、构建前

用法:
    uv run ./scripts/codegen/build_info.py
"""

import json
import sys
from datetime import datetime

from build import metadata
from cli.git import ABBREVIATED_HASH_SLICE
from cli.git import commit_count, commit_unix_time, head_hash
from common.constant.paths import BACKEND_BUILD_INFO_PATH
from common.logger import info, success
from util.sigint import graceful_sigint
from util.time import format_to_iso_8601


def generate() -> None:
    """生成并写入 build_info.json"""
    build_info = {
        "ref_name": metadata.ref_name(),
        "commit_count": commit_count(),
        "commit_hash": (commit_hash := head_hash()),
        "abbreviated_hash": commit_hash[ABBREVIATED_HASH_SLICE],
        "commit_time": format_to_iso_8601(datetime.fromtimestamp(commit_unix_time())),
        "build_time": format_to_iso_8601(datetime.now()),
    }
    compact = json.dumps(build_info, separators=(',', ':'))
    info(f"生成构建元数据：{compact}")

    BACKEND_BUILD_INFO_PATH.parent.mkdir(parents=True, exist_ok=True)
    BACKEND_BUILD_INFO_PATH.write_text(compact, encoding="utf-8")

    success(f"已生成构建元数据文件 {str(BACKEND_BUILD_INFO_PATH)!r}，大小为 {BACKEND_BUILD_INFO_PATH.stat().st_size} B")


def main() -> int:
    generate()
    return 0


if __name__ == "__main__":
    with graceful_sigint():
        sys.exit(main())
