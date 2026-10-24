#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
uv 命令封装
"""

import subprocess
from pathlib import Path

from cli import process
from common.constant.paths import PROJECT_BASE_DIR
from common.logger import warning

# 首次解析全部依赖可能较慢，给足时间
LOCK_TIMEOUT = 300
# 首次同步需要下载解释器与全部依赖
SYNC_TIMEOUT = 900


class UvError(Exception):
    """uv 无法启动，或失败原因不是调用方预期的"""
    pass


def _run(args: list[str], *, cwd: Path | None = None, timeout: float | int | None = LOCK_TIMEOUT,
         capture: bool = True, allow: tuple[int, ...] = ()) -> subprocess.CompletedProcess:
    """执行 uv 命令，失败统一包成 UvError"""
    cwd = cwd or PROJECT_BASE_DIR
    try:
        return process.run(["uv", *args], cwd=cwd, timeout=timeout, capture=capture, allow=allow)
    except process.ProcessError as error:
        if error.not_found:
            raise UvError("未找到 'uv'，请安装后加入 PATH") from error
        raise UvError(str(error)) from error


def lock(cwd: Path | None = None) -> None:
    """执行 `uv lock` 刷新 uv.lock，失败只警告，锁文件滞后不该阻断构建"""
    try:
        _run(["lock"], cwd=cwd)
    except UvError as error:
        warning(f"uv lock 未完成，uv.lock 可能滞后: {error}")


def lock_check(cwd: Path | None = None) -> bool:
    """
    执行 `uv lock --check`，判断 uv.lock 是否与项目声明一致

    :return: True 表示锁文件是最新的；False 表示需要重新 `uv lock`
    """
    # 锁过期时 uv 以 1 退出，故把 1 列入预期退出码，其余照常抛错
    result = _run(["lock", "--check"], cwd=cwd, allow=(1,))
    return result.returncode == 0


def sync_frozen(cwd: Path | None = None) -> None:
    """
    执行 `uv sync --frozen`：严格按 uv.lock 安装，绝不改写锁文件

    frozen 也跳过「锁与项目是否同步」的检查，所以调用方要先用 `lock_check()` 自行判断。

    :raise UvError: 同步失败
    """
    _run(["sync", "--frozen"], cwd=cwd, timeout=SYNC_TIMEOUT, capture=False)
