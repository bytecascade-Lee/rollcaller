#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
CI 准备入口

一次算出本次发布的全部事实，写入 GITHUB_OUTPUT：
    version —— 规范化后的版本号（去掉 v 前缀）
    draft   —— true / false
    include —— 构建矩阵（单行 JSON）

本地运行（无 GITHUB_OUTPUT）只做校验，不写文件。

用法:
    uv run ./scripts/prepare.py -v 0.1.0
    uv run ./scripts/prepare.py -v 0.1.0 --draft
"""

import argparse
import os
import sys

import util
from common.entity.version import Version, VersionError
from common.logger import info, warning
from prepare.matrix import MatrixError, build_matrix, render
from release import release_notes
from release.release_notes import ReleaseNotesAmbiguous, ReleaseNotesNotFound, ReleaseNotesSectionEmpty
from util.sigint import graceful_sigint


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="CI 准备：版本与发布说明校验、构建矩阵生成")
    parser.add_argument("-v", "--version", required=True, metavar="VERSION", help="发布版本号")
    parser.add_argument("--draft", action="store_true", help="草稿发布，跳过发布说明校验")
    return parser.parse_args()


def write_output(key: str, value: str) -> bool:
    output_file = os.environ.get("GITHUB_OUTPUT")
    if not output_file:
        warning(f"未设置 GITHUB_OUTPUT，跳过对键值对 '{key}={value}' 输出的写入")
        return False
    with open(output_file, "a", encoding="utf-8") as file:
        file.write(f"{key}={value}\n")
        info(f"追加 GitHub 输出内容 '{key}={value}' 到 {str(output_file)!r}")
    return True


def main() -> int:
    args = parse_args()

    version = Version.parse_release(args.version)
    info(f"版本校验通过: {version}")
    write_output("version", str(version))

    if args.draft:
        notes = release_notes.extract(version)
        info(f"发布说明包括 {len(notes)} 个字符")
        write_output("draft", "true")
    else:
        info(f"草稿模式，跳过发布说明")
        write_output("draft", "false")

    include = render(build_matrix())
    info(f"构建矩阵: {include}")
    write_output("include", include)

    return 0


if __name__ == "__main__":
    with graceful_sigint(on_error=util.fail):
        try:
            sys.exit(main())
        except (
                VersionError,
                MatrixError,
                ReleaseNotesNotFound,
                ReleaseNotesAmbiguous,
                ReleaseNotesSectionEmpty,
        ) as e:
            util.fail(f"准备失败: {e}")
