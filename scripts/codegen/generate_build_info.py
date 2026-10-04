#!/usr/bin/env python3
"""
生成 backend/build-info.json。

调用时机：
    1. tauri 的 beforeDevCommand / beforeBuildCommand
    2. backend/build.rs 的兜底

用法:
    uv run ./scripts/codegen/generate_build_info.py
"""

from datetime import datetime, timezone

from cli.git import get_commit_count, get_last_commit_time, is_repo, source_branch, get_head_hash
from common.constant.paths import BACKEND_DIR, PROJECT_BASE_DIR
from common.entity.build_info import BuildInfo
from common.logger import success, info
from util.sigint import quiet_sigint

OUTPUT = BACKEND_DIR / "gen" / "build-info.json"


def main() -> None:
    if not is_repo():
        branch = commit_count = commit_hash = commit_time = None
    else:
        branch = source_branch()
        commit_count = get_commit_count(cwd=PROJECT_BASE_DIR)
        commit_hash = get_head_hash(cwd=PROJECT_BASE_DIR)
        commit_time = get_last_commit_time(cwd=PROJECT_BASE_DIR)
    build_info = BuildInfo(
        branch,
        commit_count,
        commit_hash,
        commit_time=datetime.fromtimestamp(int(commit_time)) if commit_time else None,
        build_time=datetime.now(timezone.utc),
    )
    info(f"生成构建元数据：{build_info.to_json().replace(' ', '').replace('\n', ' ')})")
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    with OUTPUT.open("w", encoding="utf-8") as f:
        f.write(build_info.to_json())
        f.write("\n")
    success(f"已生成构建元数据文件 {OUTPUT}，大小为 {OUTPUT.stat().st_size} B")


if __name__ == "__main__":
    with quiet_sigint():
        main()
