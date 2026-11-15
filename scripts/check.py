#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
检查入口

顺序执行：
    前端格式 -> 前端 lint -> 前端类型 -> 后端格式 -> 后端 lint -> 脚本入口 -> 脚本格式 -> 脚本 lint -> 脚本类型

用法:
    uv run ./scripts/check.py -t x86_64-pc-windows-msvc
    uv run ./scripts/check.py --target x86_64-pc-windows-msvc
"""

import argparse
import re
import sys
from typing import Callable

from cli import basedpyright, cargo, pnpm, ruff, rustc
from common import gate
from common.constant.paths import SCRIPTS_DIR
from common.entity.target import Target, UnsupportedTargetError
from common.enums.action import Action
from common.logger import error, info, success
from util.sigint import graceful_sigint, never_interrupted
from util.str_format import join_repr

MANUAL_TOOL_DIR = SCRIPTS_DIR / "tool"
_ENTRY_POINT_RE = re.compile(r'^if __name__ == ["\']__main__["\']:', re.MULTILINE)


class CheckError(Exception):
    """仓库自身的约定被破坏"""


def _check_entry_points() -> None:
    stray = sorted(
        path.relative_to(SCRIPTS_DIR).as_posix()
        for path in SCRIPTS_DIR.rglob("*.py")
        if path.parent not in (SCRIPTS_DIR, MANUAL_TOOL_DIR)
        and _ENTRY_POINT_RE.search(path.read_text(encoding="UTF-8"))
    )
    if stray:
        raise CheckError(f"包内出现 __main__：{join_repr(stray)}；包只提供函数，需要手动触发就在根上建入口")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="检查入口：前端格式 / lint / 类型，以及后端编译")
    parser.add_argument("-t", "--target", required=True, metavar="TRIPLE", help="完整 Rust 目标三元组")

    return parser.parse_args()


def main(*, is_interrupted: Callable[[], bool] = never_interrupted) -> int:
    args = parse_args()

    target = Target(rustc.resolve_triple(args.target))
    info(f"当前目标三元组为 {target.triple!r}")

    _ = gate.ensure(target, Action.CHECK, host=rustc.host_target())

    info("检查前端格式")
    pnpm.format_check()
    if is_interrupted():
        return 0

    info("检查前端 lint")
    pnpm.lint()
    if is_interrupted():
        return 0

    info("检查前端类型")
    pnpm.check()
    if is_interrupted():
        return 0

    info("检查后端格式")
    cargo.fmt_check()
    if is_interrupted():
        return 0

    info("检查后端 lint")
    cargo.clippy(target)
    if is_interrupted():
        return 0

    info("检查脚本入口")
    _check_entry_points()
    if is_interrupted():
        return 0

    info("检查脚本格式")
    ruff.format_check()
    if is_interrupted():
        return 0

    info("检查脚本 lint")
    ruff.check()
    if is_interrupted():
        return 0

    info("检查脚本类型")
    basedpyright.check()

    success("前后端检查通过")
    return 0


if __name__ == "__main__":
    with graceful_sigint() as is_interrupted:
        try:
            sys.exit(main(is_interrupted=is_interrupted))
        except (
            gate.GateError,
            CheckError,
            basedpyright.BasedpyrightError,
            pnpm.PnpmError,
            cargo.CargoError,
            ruff.RuffError,
            rustc.RustcError,
            UnsupportedTargetError,
        ) as e:
            if not is_interrupted():
                error(f"检查失败: {e}")
                sys.exit(1)
