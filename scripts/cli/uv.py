#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
uv 命令封装
"""

from pathlib import Path

from cli._tool import Timeout, Tool
from common.constant.paths import PROJECT_BASE_DIR
from common.logger import warning


class UvError(Exception):
    """uv 无法启动，或失败原因不是调用方预期的"""

    pass


TOOL = Tool(
    "uv",
    UvError,
    cwd=PROJECT_BASE_DIR,
    timeout=Timeout.NORMAL,
    hint="请安装 uv 并加入 PATH：https://docs.astral.sh/uv/getting-started/installation/",
)


def lock(cwd: Path | None = None) -> None:
    """执行 `uv lock` 刷新 uv.lock，失败只警告，锁文件滞后不该阻断构建"""
    try:
        _ = TOOL.run(["lock"], cwd=cwd)
    except UvError as error:
        warning(f"uv lock 未完成，uv.lock 可能滞后: {error}")


def lock_check(cwd: Path | None = None) -> bool:
    """
    执行 `uv lock --check`，判断 uv.lock 是否与项目声明一致

    :return: True 表示锁文件是最新的；False 表示需要重新 `uv lock`
    """
    # 锁过期时 uv 以 1 退出，故把 1 列入预期退出码，其余照常抛错
    result = TOOL.run(["lock", "--check"], cwd=cwd, allow=(1,))
    return result.returncode == 0


def sync_frozen(cwd: Path | None = None) -> None:
    """执行 `uv sync --frozen`"""
    _ = TOOL.run(["sync", "--frozen"], cwd=cwd, timeout=Timeout.NETWORK, capture=False)
