#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
pnpm 命令封装
"""

import subprocess
from pathlib import Path

from cli import process
from cli.version import version_string
from common.constant.paths import FRONTEND_DIR

# 单条 pnpm 命令的最大超时时长（秒）
RUN_TIMEOUT = 900


class PnpmError(Exception):
    """pnpm 命令无法启动或执行失败"""


def _run(args: list[str], *, cwd: Path | None = None, timeout: float | int = RUN_TIMEOUT,
         capture: bool = True) -> subprocess.CompletedProcess:
    """执行 pnpm 命令，失败统一包成 PnpmError"""
    cwd = cwd or FRONTEND_DIR
    try:
        return process.run(["pnpm", *args], cwd=cwd, timeout=timeout, capture=capture)
    except process.ProcessError as error:
        if error.not_found:
            raise PnpmError("未找到 'pnpm'，请安装后加入 PATH") from error
        raise PnpmError(str(error)) from error


def version() -> str | None:
    """`pnpm --version` 里的版本号；输出认不出时返回 None"""
    return version_string(str(_run(["--version"]).stdout))


def install(*, cwd: Path | None = None) -> None:
    """按 lockfile 安装前端依赖"""
    _run(["install", "--frozen-lockfile"], cwd=cwd, capture=False)


def test(*, cwd: Path | None = None) -> None:
    """运行前端单元测试（`package.json` 的 test 脚本），输出直接进控制台"""
    _run(["test"], cwd=cwd, capture=False)
