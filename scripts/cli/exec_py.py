#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
执行 Python 脚本进程：用当前解释器（sys.executable）跑一个 .py 文件。

sys.executable：
    即启动本进程的解释器。入口用 `uv run python …` 启动时它就是 .venv 的解释器，
    子进程因此继承同一环境（依赖与 .venv/sitecustomize.py 都生效）；若入口是系统
    python 启动的，子进程也会是系统 python，脚本里 `from common.…` 会导入失败。

输出与阻塞：
    不捕获输出，子进程的 stdout/stderr 将直接打到当前终端。
"""

import subprocess
import sys
from pathlib import Path
from typing import Dict, Optional, Sequence

from common.logger import debug


class ExecPyError(Exception):
    """Python 脚本执行失败：解释器不可用，或脚本返回非 0"""


def run(
    script: Path,
    args: Sequence[str] = (),
    *,
    cwd: Optional[Path] = None,
    env: Optional[Dict[str, str]] = None,
) -> None:
    """执行 `<sys.executable> <script> [args…]`，阻塞到它退出；失败抛 ExecPyError。

    args：脚本自己的命令行参数，原样透传。
    cwd：None ⇒ 继承父进程的工作目录（本仓脚本都靠 paths.py 定位，不依赖 cwd）。
    env：见模块 docstring。
    """
    cmd = [sys.executable, str(script), *args]
    debug(f"执行: {' '.join(cmd)}")
    try:
        proc = subprocess.run(cmd, cwd=cwd, env=env)
    except FileNotFoundError as e:
        raise ExecPyError(f"Python 解释器不可用: {e}")
    if proc.returncode != 0:
        raise ExecPyError(f"{script.name} 失败（退出码 {proc.returncode}）")
