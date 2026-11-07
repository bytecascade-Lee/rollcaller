#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
把线上清单的 sha256 写回仓库版本索引

用法：

    uv run ./scripts/backfill_versions.py                      # 仓库索引里的全部版本
    uv run ./scripts/backfill_versions.py -v 0.8.6             # 只处理某版本
    uv run ./scripts/backfill_versions.py --check              # 只报告差异，不写文件（CI）
    uv run ./scripts/backfill_versions.py -v 0.8.6 --pr        # 回填并开 PR（CI）
"""

import argparse
import sys
from pathlib import Path

from cli import git
from common.constant.paths import VERSIONS_INDEX_SOURCE_PATH
from common.entity.version import Version, VersionError
from common.enums.source import Source
from common.logger import error, info, success
from release import backfill, github, versions_index
from util.str_format import join_repr


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="把线上清单的 sha256 回填进仓库版本索引")
    parser.add_argument("-v", "--version", nargs="+", metavar="VERSION", help="只处理这些版本")
    parser.add_argument("-s", "--source", nargs="+", choices=Source.values(), metavar="SOURCE", help="只处理这些源")
    parser.add_argument("-i", "--index", metavar="PATH", default=str(VERSIONS_INDEX_SOURCE_PATH), help="待回填的索引文件")
    parser.add_argument("--check", action="store_true", help="只报告差异，不写文件；有差异则以 1 退出")
    parser.add_argument("--pr", action="store_true", help="回填后提交到 bot 分支并开 PR")
    parser.add_argument("-t", "--timeout", type=int, default=backfill.DEFAULT_TIMEOUT, metavar="SECONDS",
                        help="单次请求超时秒数")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        wanted = [Version.parse(value) for value in args.version] if args.version else None
    except VersionError as e:
        error(f"版本号非法: {e}")
        return 1

    if args.pr and (wanted is None or len(wanted) != 1):
        error("--pr 需要且只需要一个 -v 版本")
        return 1

    sources = [Source(value) for value in (args.source or [s.value for s in backfill.DIGEST_SOURCES])]
    index_path = Path(args.index)

    try:
        entries = versions_index.read(index_path)
    except versions_index.VersionIndexError as e:
        error(f"索引不可用: {e}")
        return 1
    if not entries:
        error(f"{index_path} 是空索引")
        return 1

    scope = f"{len(wanted)} 个指定版本" if wanted else f"全部 {len(entries)} 个版本"
    info(f"索引 {index_path} | 处理 {scope} | 源 {join_repr(s.value for s in sources)}")

    try:
        merged = backfill.collect(entries, sources, wanted, args.timeout)
    except backfill.BackfillError as e:
        error(str(e))
        return 1

    if backfill.snapshot(merged) == backfill.snapshot(entries):
        success("仓库索引已与线上一致，未改动")
        return 0
    if args.check:
        error("仓库索引与线上不一致；跑一次 just backfill-versions 回填")
        return 1

    try:
        versions_index.write(index_path, merged)
    except OSError as e:
        error(f"写入 {index_path} 失败: {e}")
        return 1

    if args.pr:
        try:
            backfill.propose(wanted[0], index_path, merged)
        except (backfill.BackfillError, github.GithubReleaseError, git.GitError) as e:
            error(f"开 PR 失败: {e}")
            return 1
        success(f"已回填 {index_path} 并开 PR")
        return 0

    success(f"已回填 {index_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
