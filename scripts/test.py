#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
测试入口

顺序执行：
    刷新生成物 -> 后端测试 -> 前端测试

用法:
    uv run ./scripts/test.py -t x86_64-pc-windows-msvc
    uv run ./scripts/test.py --target x86_64-pc-windows-msvc
"""

import argparse
import sys
from typing import Callable

from cli import cargo, git, pnpm, rustc
from codegen import CodegenError, generate_all
from common import gate
from common.entity.target import Target, UnsupportedTargetError
from common.enums.action import Action
from common.logger import error, info, success
from util.sigint import graceful_sigint, never_interrupted


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="测试入口：刷新生成物后依次执行后端与前端测试")
    parser.add_argument("-t", "--target", required=True, metavar="TRIPLE", help="完整 Rust 目标三元组")

    return parser.parse_args()


def main(*, is_interrupted: Callable[[], bool] = never_interrupted) -> int:
    args = parse_args()

    target = Target(rustc.resolve_triple(args.target))
    info(f"当前目标三元组为 {target.triple!r}")

    gate.ensure(target, Action.TEST, host=rustc.host_target())

    generate_all()

    # 中断若落在代码生成期间，生成物已落盘，没必要再跑测试
    if is_interrupted():
        return 0

    # 后端先行：ts-rs 的类型导出挂在 lib 测试上，跑完才会把 .ts 写回前端
    info(f"运行后端测试: cargo test --locked --target {target.triple}")
    cargo.test(["--locked", "--target", target.triple])
    if is_interrupted():
        return 0

    info("运行前端测试: pnpm test")
    pnpm.test()

    success("前后端测试通过")
    return 0


if __name__ == "__main__":
    with graceful_sigint() as is_interrupted:
        try:
            sys.exit(main(is_interrupted=is_interrupted))
        except (
                CodegenError,
                gate.GateError,
                git.GitError,
                pnpm.PnpmError,
                rustc.RustcError,
                cargo.CargoError,
                UnsupportedTargetError,
        ) as e:
            if not is_interrupted():
                error(f"测试失败: {e}")
                sys.exit(1)
