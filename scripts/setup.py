#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
开发环境入口：体检与初始化

用法:
    uv run ./scripts/setup.py            # 体检通过后写 git 配置、装依赖（just setup）
    uv run ./scripts/setup.py --check    # 只体检，不修改任何东西（just doctor）
"""

import argparse
import sys

from common.logger import error, success
from prepare.environment import EnvError, check, setup
from util.sigint import graceful_sigint


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="开发环境入口：体检与初始化")
    parser.add_argument("--check", action="store_true", help="只体检，不修改任何东西")
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    if not args.check:
        setup()
        return 0

    problems = check()
    if not problems:
        success("体检通过")
        return 0

    for problem in problems:
        error(problem)
    error(f"体检未通过，共 {len(problems)} 项")
    return 1


if __name__ == "__main__":
    with graceful_sigint() as is_interrupted:
        try:
            sys.exit(main())
        except EnvError as e:
            if not is_interrupted():
                error(f"环境准备失败: {e}")
                sys.exit(1)
