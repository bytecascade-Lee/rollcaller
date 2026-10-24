#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
开发环境入口

串行刷新生成物，启动 tauri dev，前端由 beforeDevCommand 拉起

用法:
    uv run ./scripts/dev.py -t x86_64-pc-windows-msvc
    uv run ./scripts/dev.py --target aarch64-pc-windows-msvc
"""

import argparse
import sys
from typing import Callable

from cli import git, rust, tauri
from codegen import CodegenError, generate_all
from common import gate
from common.entity.target import Target, UnsupportedTargetError
from common.enums.action import Action
from common.logger import error, info
from util.sigint import graceful_sigint, never_interrupted


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="开发环境入口：生成代码，然后执行 tauri dev")
    parser.add_argument("-t", "--target", metavar="TRIPLE", help="完整三元组构建目标")
    return parser.parse_args()


def main(*, is_interrupted: Callable[[], bool] = never_interrupted) -> None:
    args = parse_args()

    target = Target(rust.resolve_triple(args.target))
    info(f"当前目标三元组为 {target.triple!r}")

    gate.ensure(target, Action.BUILD, host=rust.get_host_target())

    generate_all()

    # 中断若落在代码生成期间，生成物已落盘，但不应再拉起 dev server
    if is_interrupted():
        return

    tauri.dev(target)


if __name__ == "__main__":
    with graceful_sigint(exit_code=0) as is_interrupted:
        try:
            main(is_interrupted=is_interrupted)
        except (
                CodegenError,
                gate.GateError,
                git.GitError,
                rust.RustError,
                tauri.TauriError,
                UnsupportedTargetError,
        ) as e:
            if not is_interrupted():
                error(f"开发环境启动失败: {e}")
                sys.exit(1)
