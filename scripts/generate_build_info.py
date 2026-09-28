#!/usr/bin/env python3
"""
生成 backend/build-info.json —— 构建信息载体（Rust 侧 AppInfo 的数据来源）。

调用时机：
    1. tauri 的 beforeDevCommand / beforeBuildCommand（正常路径，每次 dev/build 都刷新）
    2. backend/build.rs 的兜底（仅在文件不存在时，兜住不经 tauri 的 cargo 调用）

字段（全部字符串，时间为 ISO 8601 且显式带本机时区偏移）：
    branch        来源分支：tag 发布取发布线；手动发布取所选 ref；本地取当前分支，
                  detached HEAD 时为 detached@<tag> 或 detached@<短哈希>
    commit_count  HEAD 的提交总数
    short_hash    HEAD 的短哈希
    commit_time   HEAD 的提交时间
    build_time    本次生成时间

为什么是这个分层（见 docs/architect/自动更新架构-三段式管线与发布链路.md）：
    - debug 构建由 app_info 运行时读本文件 ⇒ 不进入编译输入，既不触发重编译、又能每次
      dev 都拿到新值（代价仅是文件不存在时回落 unknown）；
    - release 构建由 include_str! 编译期嵌入 ⇒ 产物自包含，不依赖运行时文件。

用法:
    uv run python scripts/generate_build_info.py
"""

import json
import os
import time
from datetime import datetime, timezone
from pathlib import Path

from common.git import get_branch, get_commit_count, get_head_hash, git
from common.logger import log
from common.release import RELEASE_LINE

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "backend" / "build-info.json"


def to_local_iso(ts: int) -> str:
    """Unix 时间戳 → ISO 8601 字符串，显式转换为本机时区（如 2026-09-28T12:40:12+08:00）"""
    return datetime.fromtimestamp(ts, tz=timezone.utc).astimezone().isoformat(timespec="seconds")


def source_branch() -> str:
    """
    来源分支：
        tag 发布    → 发布线（tag 位于其历史由 check_release_source.py 在 validate 阶段断言）
        手动发布    → 所选 ref（validate 阶段已确认与输入框一致）
        本地/其它   → 当前分支，detached HEAD 时形如 detached@<tag|短哈希>
    """
    ref_type = os.environ.get("GITHUB_REF_TYPE")
    if ref_type == "tag":
        return RELEASE_LINE
    ref_name = os.environ.get("GITHUB_REF_NAME")
    if ref_type == "branch" and ref_name:
        return ref_name
    return get_branch(ROOT)


def main() -> None:
    info = {
        "branch": source_branch(),
        "commit_count": str(get_commit_count(ROOT)),
        "short_hash": get_head_hash(short=True, cwd=ROOT),
        "commit_time": to_local_iso(int(git(["log", "-1", "--format=%ct"], ROOT))),
        "build_time": to_local_iso(int(time.time())),
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    # 固定 LF：生成物不应随平台变化，且它不在版本控制内、无需迁就行尾约定
    with OUTPUT.open("w", encoding="utf-8", newline="\n") as f:
        json.dump(info, f, ensure_ascii=False, indent=4)
        f.write("\n")
    log("INFO", f"已生成 {OUTPUT.relative_to(ROOT)}：{json.dumps(info, ensure_ascii=False)}")


if __name__ == "__main__":
    main()
