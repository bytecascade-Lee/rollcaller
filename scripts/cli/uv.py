#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
uv 命令封装
"""

import subprocess
from pathlib import Path

from common.constant.paths import PROJECT_BASE_DIR
from common.logger import debug


class UvError(Exception):
    """uv 调用失败"""


def lock(cwd: Path | None = None) -> None:
    """执行 `uv lock` 刷新 uv.lock，启动失败或退出码非 0 抛 UvError"""
    workdir = cwd or PROJECT_BASE_DIR
    cmd = ["uv", "lock"]
    debug(f"执行: {' '.join(cmd)} (cwd={workdir})")
    try:
        proc = subprocess.run(cmd, cwd=workdir, capture_output=True, check=False)
    except FileNotFoundError as e:
        raise UvError(f"未找到 uv 命令: {e}")
    if proc.returncode != 0:
        stderr = proc.stderr.decode("utf-8", "replace").strip()
        raise UvError(f"uv lock 失败（退出码 {proc.returncode}）: {stderr}")
