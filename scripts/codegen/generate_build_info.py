#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
生成 backend/gen/build/build_info.json，构建元数据的唯一载体

调用时机：
    1. 开发前
    2. 构建前

用法:
    uv run ./scripts/codegen/generate_build_info.py
"""

import json
import sys
from datetime import UTC, datetime

from build import metadata
from cli.git import commit_count, head_hash, commit_unix_time
from common.constant.paths import BACKEND_BUILD_INFO_PATH
from common.entity.build_info import BuildInfo
from common.logger import info, success
from util.sigint import quiet_sigint


def generate_build_info() -> BuildInfo:
    """收集本次构建的 git 元数据"""
    return BuildInfo(
        ref_name=metadata.ref_name(),
        commit_count=commit_count(),
        commit_hash=head_hash(),
        commit_time=datetime.fromtimestamp(commit_unix_time(), UTC),
        build_time=datetime.now(UTC),
    )


def to_json_dict(build_info: BuildInfo) -> dict[str, str | int]:
    return {
        "ref_name": build_info.ref_name,
        "commit_count": build_info.commit_count,
        "commit_hash": build_info.commit_hash,
        "abbreviated_hash": build_info.abbreviated_hash,
        "commit_time": build_info.commit_time.isoformat(timespec="milliseconds"),
        "build_time": build_info.build_time.isoformat(timespec="milliseconds"),
    }


def main() -> int:
    build_info = generate_build_info()
    info(f"生成构建元数据：{build_info}")

    BACKEND_BUILD_INFO_PATH.parent.mkdir(parents=True, exist_ok=True)
    with BACKEND_BUILD_INFO_PATH.open("w", encoding="utf-8") as f:
        json.dump(to_json_dict(build_info), f, indent=4)
        f.write("\n")
    success(f"已生成构建元数据文件 {str(BACKEND_BUILD_INFO_PATH)!r}，大小为 {BACKEND_BUILD_INFO_PATH.stat().st_size} B")

    return 0


if __name__ == "__main__":
    with quiet_sigint():
        sys.exit(main())
