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

import util
from cli import exec_py, rust, tauri
from codegen import CODEGEN_FILES
from common.entity.target import Target
from common.logger import info
from util.sigint import graceful_sigint


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="开发环境入口：生成代码，然后执行 tauri dev")
    parser.add_argument("-t", "--target", metavar="TRIPLE", help="完整三元组构建目标")
    return parser.parse_args()


def main() -> None:
    args = parse_args()

    target = Target(rust.resolve_triple(args.target))
    info(f"当前目标三元组为 {target.triple!r}")

    for script in CODEGEN_FILES:
        info(f"执行 {script.name} 生成代码...")
        exec_py.run(script)

    tauri.dev(target)


if __name__ == "__main__":
    with graceful_sigint(on_error=util.fail, exit_code=0):
        main()
