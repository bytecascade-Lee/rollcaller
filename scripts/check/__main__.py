#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
发布前检查入口

用法:
    uv run python -m check -v 0.1.0
    uv run python -m check -v 0.1.0 --draft   # 草稿跳过发布说明
"""

import argparse
import os
import sys

import util
from check.check_notes import check_notes
from check.check_version import check_version
from common.entity.version import VersionError
from common.logger import info
from release.release_notes import ReleaseNotesAmbiguous, ReleaseNotesSectionEmpty, ReleaseNotesNotFound
from util.sigint import graceful_sigint


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="发布前检查版本是否可解析、发布说明是否可正常提取")
    parser.add_argument("-v", "--version", required=True, metavar="VERSION", help="发布版本号")
    parser.add_argument("--draft", action="store_true", help="草稿发布，跳过发布说明检查")
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    version = check_version(args.version)
    notes = check_notes(version, draft=args.draft)

    github_output = os.environ.get("GITHUB_OUTPUT")
    if github_output:
        with open(github_output, "a", encoding="utf-8") as file:
            file.write(f"version={version}\n")

    if args.draft:
        info(f"版本检查通过: {version}，草稿模式，跳过发布说明")
    else:
        info(f"版本检查通过: {version}，版本发布说明包含 {len(notes)} 个字符")
    return 0


if __name__ == "__main__":
    with graceful_sigint(on_error=util.fail):
        try:
            sys.exit(main())
        except (
                VersionError,
                ReleaseNotesNotFound,
                ReleaseNotesAmbiguous,
                ReleaseNotesSectionEmpty,
        ) as error:
            util.fail(f"发布检查失败: {error}")
