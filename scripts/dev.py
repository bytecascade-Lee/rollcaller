#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
开发环境入口：先串行刷新生成物，再启动 tauri dev，前端由 beforeDevCommand 拉起。

用法:
    uv run ./scripts/dev.py
    uv run ./scripts/dev.py --target arm64

"""

import argparse
import signal
import sys
from pathlib import Path
from typing import Tuple

import util
from cli import exec_py, tauri
from cli.exec_py import ExecPyError
from cli.tauri import TauriError
from common.constant.paths import SCRIPTS_DIR
from common.logger import info, warning
from common.targets import TargetError, host_target, parse_target

CODEGEN_DIR = SCRIPTS_DIR / "codegen"

# 启动前必须刷新的一批生成物，顺序即依赖顺序
CODEGEN: Tuple[Tuple[str, Path], ...] = (
    ("routing entries", CODEGEN_DIR / "generate_routing_entries.py"),
    ("build info", CODEGEN_DIR / "generate_build_info.py"),
)

EXIT_INTERRUPTED = 0

_interrupted = False


def on_sigint(_signum: int, _frame: object) -> None:
    """Ctrl+C 的第一响应：只置标志、不抛异常。

    子进程与我们挂在同一个 console 上，会同时收到 CTRL_C_EVENT 各自退出；这里刻意
    不打断 subprocess 的等待，好让它们退干净再收场——否则父进程先走，终端会被子进程
    的收尾输出继续占用。再按一次则不再拦，直接放行 KeyboardInterrupt 强制退出。
    """
    global _interrupted
    if _interrupted:
        raise KeyboardInterrupt
    _interrupted = True
    warning("执行 Ctrl+C，等待子进程退出...")


def _fail(message: str) -> None:
    """失败出口：用户主动中断时静默收场，不把中断报成错误。"""
    if _interrupted:
        sys.exit(EXIT_INTERRUPTED)
    util.fail(message)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="开发环境入口：生成代码 → tauri dev")
    parser.add_argument(
        "--target",
        metavar="TRIPLE",
        help="构建目标（架构别名或完整三元组）；缺省为本机三元组",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()

    try:
        target = parse_target(args.target) if args.target else host_target()
    except TargetError as e:
        _fail(str(e))

    for label, script in CODEGEN:
        info(f"生成 {label}…")
        try:
            exec_py.run(script)
        except ExecPyError as e:
            _fail(str(e))

    try:
        tauri.dev(target)
    except TauriError as e:
        _fail(str(e))


if __name__ == "__main__":
    signal.signal(signal.SIGINT, on_sigint)
    try:
        main()
    except KeyboardInterrupt:
        pass
    if _interrupted:
        sys.exit(EXIT_INTERRUPTED)
