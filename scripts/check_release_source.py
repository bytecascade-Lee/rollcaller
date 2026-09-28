#!/usr/bin/env python3
"""
发布来源校验：在 release.yaml 的 validate 阶段运行，不满足即失败（fail fast）。

校验两件事：
    1. 手动发布（workflow_dispatch）：输入的 branch 必须与实际触发的 ref 一致
       （忽略大小写与首尾空白）。这是「确认」而非「白名单」—— 从 hotfix 等分支
       发版是合法需求，脚本只负责拦住手滑选错 ref 的情况。
    2. tag 发布：tag 所属提交必须位于发布线的历史上 —— 让 build_ci.py 的
       「tag 发布 ⇒ 来源分支为发布线」成为被校验的事实，而不是假设。
       （tag_release.py 已要求打 tag 时处于 master/main，但手工 git tag 可绕过。）

环境变量（GitHub Actions 注入）：
    GITHUB_EVENT_NAME   workflow_dispatch / push
    GITHUB_REF_NAME     所选 ref 名（分支名或 tag 名）
    GITHUB_REF_TYPE     branch / tag
    GITHUB_SHA          触发提交
    INPUT_BRANCH        手动发布的输入（仅该事件有值）

用法:
    uv run python scripts/check_release_source.py
"""

import os

from common.git import GitError, git
from common.logger import log
from common.release import RELEASE_LINE


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def check_dispatch_ref(ref_name: str) -> None:
    """手动发布：输入的 ref 名必须与所选 ref 一致（忽略大小写与首尾空白）"""
    typed = (os.environ.get("INPUT_BRANCH") or "").strip()
    if not typed:
        fail("手动发布必须填写 branch 输入（与所选 ref 完全一致的分支名）")
    if typed.lower() != ref_name.strip().lower():
        fail(
            f"输入的分支名 [{typed}] 与实际触发的 ref [{ref_name}] 不一致。\n"
            f"请核对本次运行上方选择的 ref，或修正输入后重试。"
        )
    log("INFO", f"发布来源确认一致：{ref_name}")


def check_tag_on_release_line(sha: str) -> None:
    """tag 发布：断言该提交位于发布线历史上"""
    remote_ref = f"refs/remotes/origin/{RELEASE_LINE}"
    try:
        git(["fetch", "--no-tags", "origin", f"{RELEASE_LINE}:{remote_ref}"])
    except GitError as e:
        fail(f"无法获取发布线 {RELEASE_LINE}（校验 tag 归属所需）: {e}")
    try:
        git(["merge-base", "--is-ancestor", sha, remote_ref])
    except GitError:
        fail(
            f"tag 所属提交 {sha[:12]} 不在发布线 {RELEASE_LINE} 的历史上。\n"
            f"发布 tag 必须打在 {RELEASE_LINE}：请先将其合入 {RELEASE_LINE} 再打 tag。"
        )
    log("INFO", f"tag 位于发布线 {RELEASE_LINE} 的历史上：{sha[:12]}")


def main() -> None:
    event = os.environ.get("GITHUB_EVENT_NAME", "")
    ref_name = os.environ.get("GITHUB_REF_NAME", "")
    ref_type = os.environ.get("GITHUB_REF_TYPE", "")
    sha = os.environ.get("GITHUB_SHA", "")

    if event == "workflow_dispatch":
        check_dispatch_ref(ref_name)
    elif ref_type == "tag":
        if not sha:
            fail("缺少 GITHUB_SHA，无法校验 tag 是否位于发布线上")
        check_tag_on_release_line(sha)
    else:
        log("WARN", f"未识别的事件（event={event} ref_type={ref_type}），跳过发布来源校验")


if __name__ == "__main__":
    main()
