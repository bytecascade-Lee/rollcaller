#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
检查入口

顺序执行：
    后端格式 -> 前端格式 -> 前端 lint -> 前端类型 -> 后端编译

用法:
    uv run ./scripts/check.py -t x86_64-pc-windows-msvc
    uv run ./scripts/check.py --target x86_64-pc-windows-msvc
"""

import argparse
import sys
from typing import Callable

from cli import cargo, pnpm, rustc
from common import gate
from common.entity.target import Target, UnsupportedTargetError
from common.enums.action import Action
from common.logger import error, info, success
from util.sigint import graceful_sigint, never_interrupted


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="检查入口：前端格式 / lint / 类型，以及后端编译")
    parser.add_argument("-t", "--target", required=True, metavar="TRIPLE", help="完整 Rust 目标三元组")

    return parser.parse_args()


def main(*, is_interrupted: Callable[[], bool] = never_interrupted) -> int:
    args = parse_args()

    target = Target(rustc.resolve_triple(args.target))
    info(f"当前目标三元组为 {target.triple!r}")

    gate.ensure(target, Action.CHECK, host=rustc.host_target())

    info("检查后端格式: cargo fmt --check")
    cargo.fmt_check()
    if is_interrupted():
        return 0

    info("检查前端格式: pnpm format:check")
    pnpm.format_check()
    if is_interrupted():
        return 0

    info("检查前端 lint: pnpm lint")
    pnpm.lint()
    if is_interrupted():
        return 0

    info("检查前端类型: pnpm check")
    pnpm.check()
    if is_interrupted():
        return 0

    info(f"检查后端编译: cargo check --locked --target {target.triple}")
    cargo.check(["--locked", "--target", target.triple])

    success("前后端检查通过")
    return 0


if __name__ == "__main__":
    with graceful_sigint() as is_interrupted:
        try:
            sys.exit(main(is_interrupted=is_interrupted))
        except (
                gate.GateError,
                pnpm.PnpmError,
                cargo.CargoError,
                rustc.RustcError,
                UnsupportedTargetError,
        ) as e:
            if not is_interrupted():
                error(f"检查失败: {e}")
                sys.exit(1)
