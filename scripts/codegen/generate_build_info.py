#!/usr/bin/env python3
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

from cli.git import get_commit_count, get_head_hash, get_last_commit_time, is_repo, source_branch, GitError
from common.constant.paths import BACKEND_BUILD_INFO_PATH, PROJECT_BASE_DIR
from common.entity.build_info import BuildInfo
from common.logger import info, success
from util.sigint import quiet_sigint


def generate_build_info() -> BuildInfo:
    """收集本次构建的 git 元数据；不在仓库内或仓库无提交时失败"""
    if not is_repo(PROJECT_BASE_DIR):
        raise GitError(f"{PROJECT_BASE_DIR} 不是 Git 仓库：构建元数据只能在仓库内生成")

    commit_time = get_last_commit_time(cwd=PROJECT_BASE_DIR)
    if commit_time is None:
        raise GitError("当前仓库没有任何提交，无法确定构建来源")

    return BuildInfo(
        ref_name=source_branch(),
        commit_count=get_commit_count(cwd=PROJECT_BASE_DIR),
        commit_hash=get_head_hash(cwd=PROJECT_BASE_DIR),
        commit_time=datetime.fromtimestamp(int(commit_time), UTC),
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
