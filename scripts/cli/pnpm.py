#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
pnpm 命令封装
"""

import subprocess
from pathlib import Path

from cli import process
from common.constant.paths import FRONTEND_DIR

# 首次安装需要下载并链接全部依赖
INSTALL_TIMEOUT = 900


class PnpmError(Exception):
    """pnpm 无法启动，或安装失败"""
    pass


def _run(args: list[str], *, cwd: Path | None = None, timeout: float | int = INSTALL_TIMEOUT,
         capture: bool = True) -> subprocess.CompletedProcess:
    """执行 pnpm 命令，失败统一包成 PnpmError"""
    cwd = cwd or FRONTEND_DIR
    try:
        return process.run(["pnpm", *args], cwd=cwd, timeout=timeout, capture=capture)
    except process.ProcessError as error:
        if error.not_found:
            raise PnpmError("未找到 'pnpm'，请安装后加入 PATH") from error
        raise PnpmError(str(error)) from error


def install(*, cwd: Path | None = None) -> None:
    """按 lockfile 安装前端依赖"""
    _run(["install", "--frozen-lockfile"], cwd=cwd, capture=False)
