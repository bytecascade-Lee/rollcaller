#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
开发环境入口

串行刷新生成物，启动 tauri dev，前端由 beforeDevCommand 拉起

用法:
    uv run ./scripts/dev.py
    uv run ./scripts/dev.py --target x86_64-pc-windows-msvc

"""

import argparse
from pathlib import Path
from typing import List, Tuple

import util
from cli import exec_py, rust, tauri
from common.constant.paths import SCRIPTS_DIR
from common.entity.target import Target
from common.logger import info
from util.sigint import graceful_sigint

CODEGEN_DIR = SCRIPTS_DIR / "codegen"

# 启动前必须刷新的一批生成物，顺序即依赖顺序
CODEGEN: List[Tuple[str, Path]] = [
    ("routing entries", CODEGEN_DIR / "generate_routing_entries.py"),
    ("build info", CODEGEN_DIR / "generate_build_info.py"),
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="开发环境入口：生成代码 → tauri dev")
    parser.add_argument(
        "--target",
        metavar="TRIPLE",
        help="完整三元组构建目标；缺省为本机三元组",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()

    target = Target(rust.resolve_triple(args.target))
    info(f"当前目标三元组为 {target.triple!r}")

    for label, script in CODEGEN:
        info(f"生成 {label}…")
        exec_py.run(script)

    tauri.dev(target)


if __name__ == "__main__":
    with graceful_sigint(on_error=util.fail, exit_code=0):
        main()
