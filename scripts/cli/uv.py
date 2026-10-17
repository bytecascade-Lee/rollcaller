#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
uv 命令封装
"""

import subprocess
from pathlib import Path

from common.constant.paths import PROJECT_BASE_DIR
from common.logger import debug, warning


# class UvError(Exception):
#     """uv 调用失败"""
#     pass


def lock(cwd: Path | None = None) -> None:
    """执行 `uv lock` 刷新 uv.lock"""
    workdir = cwd or PROJECT_BASE_DIR
    cmd = ["uv", "lock"]
    debug(f"执行: {' '.join(cmd)} (cwd={workdir})")
    try:
        proc = subprocess.run(cmd, cwd=workdir, capture_output=True, check=False)
    except FileNotFoundError as e:
        warning(f"未找到 uv 命令: {e}")
        exit(code=0)
    if proc.returncode != 0:
        stderr = proc.stderr.decode("utf-8", "replace").strip()
        warning(f"uv lock 失败（退出码 {proc.returncode}）: {stderr}")
