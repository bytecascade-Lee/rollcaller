#!/usr/bin/env python3
"""
创建发布 tag：git tag -a v{版本号} -m <信息>（默认 "Release v{版本号}"）。

前置校验（不满足则报错退出，不打 tag）:
    1. versions.json 中必须已包含该版本 —— tag 带字母 v，versions.json 不带 v，
       本脚本自动做归一化比对（v0.7.0 与 0.7.0 视为同一版本）
    2. 本地不存在同名 tag（避免误覆盖）

用法:
    uv run scripts/tag_release.py <版本号> [-m <tag 信息>]

默认 tag 信息为 "Release v{版本号}"（如 v0.7.0 → "Release v0.7.0"）。
"""
import argparse
import json
from pathlib import Path

from common import version as version_mod
from common.git import GitError, git, get_branch
from common.logger import log

ROOT = Path(__file__).resolve().parent.parent

# 版本索引源文件（仓库维护，发布期唯一标定 severity/force 的地方）
VERSIONS_INDEX_PATH = ROOT / "resources" / "update" / "versions.json"


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def index_versions() -> set:
    """读取 versions.json 中已标定的版本号集合（规范化，不含前导 v）。"""
    if not VERSIONS_INDEX_PATH.exists():
        fail(f"缺少版本索引源文件: {VERSIONS_INDEX_PATH}")
    try:
        data = json.loads(VERSIONS_INDEX_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        fail(f"versions.json 解析失败: {e}")
    entries = data.get("versions")
    if not isinstance(entries, list):
        fail("versions.json 缺少 versions 数组")
    return {e.get("version") for e in entries if isinstance(e, dict)}


def main() -> None:
    parser = argparse.ArgumentParser(
        description="创建发布 tag（git tag -a v{版本} -m ...），打 tag 前校验 versions.json 已标定该版本"
    )
    parser.add_argument(
        "version",
        help="语义化版本号，可带 v 也可不带，例如 v0.7.0 或 0.7.0",
    )
    parser.add_argument(
        "-m",
        "--message",
        help="tag 提交信息（默认: Release v{版本号}）",
    )
    args = parser.parse_args()

    # 必须在 master 或 main 分支
    branch = get_branch()
    if branch != "master" or branch != "main":
        fail("必须在 master 或 main 分支打tag")

    # 去除前导 v + 格式校验；tag 带 v，versions.json 不带 v
    try:
        ver = version_mod.validate(args.version)
    except version_mod.InvalidVersionError as e:
        fail(str(e))
    tag = f"v{ver}"
    message = args.message or f"Release {tag}"

    # 1. versions.json 必须已标定该版本
    versions = index_versions()
    if ver not in versions:
        fail(
            f"versions.json 中未标定版本 {ver}（tag: {tag}），无法打 tag。\n"
            f"请先标定: python scripts/update_versions_index.py {ver} "
            f"[--severity normal|important|critical] [--force]"
        )

    # 2. 本地不得已有同名 tag
    try:
        existing = git(["tag", "--list", tag])
    except GitError as e:
        fail(str(e))
    if existing:
        fail(f"本地 tag {tag} 已存在，如需重建请先删除: git tag -d {tag}")

    # 3. 打 annotated tag
    try:
        git(["tag", "-a", tag, "-m", message])
    except GitError as e:
        fail(str(e))

    log("INFO", f"已创建 tag {tag}（{message}）")


if __name__ == "__main__":
    main()
