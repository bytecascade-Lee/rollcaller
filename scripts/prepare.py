#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
CI 准备入口

计算并写入下列信息到 GITHUB_OUTPUT：
    version 规范化后的版本号
    draft   true / false
    matrix  构建矩阵

用法:
    uv run ./scripts/prepare.py -v 0.1.0
    uv run ./scripts/prepare.py -v 0.1.0 --draft
"""

import argparse
import os
import sys

from common.entity.version import Version, VersionError
from common.logger import error, info, warning
from prepare.matrix import MatrixError, build_matrix
from release import release_notes
from release.release_notes import ReleaseNotesAmbiguous, ReleaseNotesNotFound, ReleaseNotesSectionEmpty
from util.sigint import graceful_sigint


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="CI 准备：版本与发布说明校验、构建矩阵生成")
    parser.add_argument("-v", "--version", required=True, metavar="VERSION", help="发布版本号")
    parser.add_argument("--draft", action="store_true", help="草稿发布，跳过发布说明校验")
    return parser.parse_args()


def write_output(key: str, value: str) -> None:
    output_file = os.environ.get("GITHUB_OUTPUT")
    if not output_file:
        warning(f"未设置 GITHUB_OUTPUT，跳过对键值对 '{key}={value}' 输出的写入")
        return
    with open(output_file, "a", encoding="utf-8") as file:
        file.write(f"{key}={value}\n")
        info(f"追加 GitHub 输出内容 '{key}={value}' 到 {str(output_file)!r}")


def main() -> int:
    args = parse_args()

    version = Version.parse(args.version, erase_build_metadata=True)
    info(f"版本校验通过: {version}")
    write_output("version", str(version))

    if args.draft:
        info(f"草稿模式，跳过发布说明的检查")
        write_output("draft", "true")
    else:
        notes = release_notes.extract(version)
        info(f"版本 {version!r} 对应的发布说明包括 {len(notes.splitlines())} 行，{len(notes)} 个字符")
        write_output("draft", "false")

    matrix = build_matrix()
    info(f"构建矩阵: {matrix}")
    write_output("matrix", matrix)

    return 0


if __name__ == "__main__":
    with graceful_sigint():
        try:
            sys.exit(main())
        except (
                VersionError,
                MatrixError,
                ReleaseNotesNotFound,
                ReleaseNotesAmbiguous,
                ReleaseNotesSectionEmpty,
        ) as e:
            error(f"准备失败: {e}")
            sys.exit(1)
