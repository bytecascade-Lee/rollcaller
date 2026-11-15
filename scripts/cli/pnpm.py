#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
pnpm 命令封装
"""

from pathlib import Path

from cli._tool import Timeout, Tool
from common.constant.paths import FRONTEND_DIR


class PnpmError(Exception):
    """pnpm 命令无法启动或执行失败"""


TOOL = Tool(
    "pnpm",
    PnpmError,
    cwd=FRONTEND_DIR,
    timeout=Timeout.NORMAL,
    hint="请安装 pnpm 并加入 PATH：https://pnpm.io/installation",
)


def install(*, cwd: Path | None = None) -> None:
    """按 lockfile 安装前端依赖"""
    _ = TOOL.run(["install", "--frozen-lockfile"], cwd=cwd, timeout=Timeout.EXTENDED, capture=False)


def test(*, cwd: Path | None = None) -> None:
    """运行前端单元测试（`package.json` 的 test 脚本），输出直接进控制台"""
    _ = TOOL.run(["test"], cwd=cwd, capture=False)


def lint(*, cwd: Path | None = None) -> None:
    """运行 ESLint（`package.json` 的 lint 脚本），输出直接进控制台"""
    _ = TOOL.run(["lint"], cwd=cwd, capture=False)


def typecheck(*, cwd: Path | None = None) -> None:
    """运行 svelte-check 类型检查（`package.json` 的 check 脚本），输出直接进控制台"""
    _ = TOOL.run(["check"], cwd=cwd, capture=False)


def format(*, check: bool = False, cwd: Path | None = None) -> None:
    """
    格式化前端：写文件走 `format` 脚本，只检查走 `format:check` 脚本

    :param check: True 时只检查不写文件
    """
    _ = TOOL.run(["format:check" if check else "format"], cwd=cwd, capture=False)
