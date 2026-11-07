#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
发布后自检入口，逐源 × 逐版本核对线上清单与版本索引

用法：

    uv run ./scripts/verify_release.py                        # 两端在册的全部版本
    uv run ./scripts/verify_release.py -v 0.8.5               # 只校验某版本
    uv run ./scripts/verify_release.py -s cnb                 # 只校验某源
    uv run ./scripts/verify_release.py -s develop             # 校验本地发布服务
    uv run ./scripts/verify_release.py -i resources/update/versions.json   # 用仓库索引代替线上索引
"""

import argparse
import sys
from pathlib import Path

from common.entity.version import Version, VersionError
from common.enums.source import Source
from common.logger import error, info, success
from release import audit, versions_index


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="发布后自检：线上清单 / 版本索引的一致性")
    parser.add_argument("-v", "--version", metavar="VERSION", help="只校验该版本（缺省校验索引里的全部版本）")
    parser.add_argument("-s", "--source", nargs="+", choices=Source.values(), metavar="SOURCE",
                        help=f"只校验这些源（缺省 {' '.join((Source.GITHUB.value, Source.CNB.value))}）")
    parser.add_argument("-t", "--timeout", type=int, default=audit.DEFAULT_TIMEOUT, metavar="SECONDS",
                        help="单次请求超时秒数")
    parser.add_argument("--retry", type=int, default=3, metavar="N",
                        help="线上索引里还没有目标版本时最多尝试几次（默认 3）")
    parser.add_argument("-i", "--index", metavar="PATH",
                        help="用这份本地版本索引（如 resources/update/versions.json）代替线上索引，"
                             "用来在发布前验证回填的摘要")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        version = Version.parse(args.version) if args.version else None
    except VersionError as e:
        error(f"版本号非法: {e}")
        return 1

    sources = [Source(value) for value in (args.source or (Source.GITHUB.value, Source.CNB.value))]
    try:
        key_id = audit.expected_key_id()
        local_entries = None
        if args.index:
            local_entries = versions_index.read(Path(args.index))
            info(f"索引来源: {args.index}（本地文件，{len(local_entries)} 个版本）")
    except (audit.AuditError, versions_index.VersionIndexError) as e:
        error(f"自检无法进行: {e}")
        return 1
    info(f"内置公钥 key id: {key_id} | 校验源: {'、'.join(s.value for s in sources)} | 版本: {version or '在册全部'}")

    failures: list[audit.Failure] = []
    for source in sources:
        found = audit.check_source(source, version, key_id, args.timeout, args.retry, local_entries)
        if found:
            failures.extend(found)
        else:
            success(f"[{source.value}] 全部通过")

    if not failures:
        success("发布后自检全部通过")
        return 0

    for failure in failures:
        error(failure.render())
    error(f"共有 {len(failures)} 项失败")
    return 1


if __name__ == "__main__":
    sys.exit(main())
