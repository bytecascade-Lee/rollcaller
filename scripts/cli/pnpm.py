#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
pnpm 命令封装
"""

from cli.tool import Timeout, Tool
from common.constant.paths import FRONTEND_DIR


class PnpmError(Exception):
    """pnpm 命令无法启动或执行失败"""


# 工作目录由描述符固定：cwd 是执行环境而不是命令输入，故下面的函数一律不再暴露该参数。
TOOL = Tool(
    "pnpm",
    PnpmError,
    cwd=FRONTEND_DIR,
    timeout=Timeout.NORMAL,
    hint="请安装 pnpm 并加入 PATH：https://pnpm.io/installation",
)


def install() -> None:
    """按 lockfile 安装前端依赖"""
    _ = TOOL.run(["install", "--frozen-lockfile"], timeout=Timeout.EXTENDED, capture=False)


def test() -> None:
    """运行前端单元测试（`package.json` 的 test 脚本），输出直接进控制台"""
    _ = TOOL.run(["test"], capture=False)


def lint() -> None:
    """运行 ESLint（`package.json` 的 lint 脚本），输出直接进控制台"""
    _ = TOOL.run(["lint"], capture=False)


def typecheck() -> None:
    """运行 svelte-check 类型检查（`package.json` 的 check 脚本），输出直接进控制台"""
    _ = TOOL.run(["check"], capture=False)


def licenses() -> str:
    """生产依赖的许可证清单，JSON 格式：`pnpm licenses list --prod --json`"""
    return TOOL.text(["licenses", "list", "--prod", "--json"])


def format(*, check: bool = False) -> None:
    """
    格式化前端：写文件走 `format` 脚本，只检查走 `format:check` 脚本

    :param check: True 时只检查不写文件
    """
    _ = TOOL.run(["format:check" if check else "format"], capture=False)
