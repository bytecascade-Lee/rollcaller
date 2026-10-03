#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""rustc 命令封装"""

import subprocess
from functools import cache
from pathlib import Path

from common.logger import debug


class RustError(Exception):
    """rustc 调用失败"""
    pass


def _run(args: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess:
    """执行 rustc 命令，失败抛 RustError"""
    debug(f"执行: rustc {' '.join(args)}")
    try:
        result = subprocess.run(
            ["rustc", *args],
            cwd=cwd,
            capture_output=True,
            check=False,
        )
    except FileNotFoundError:
        raise RustError("未找到 rustc 命令，请确保已安装 Rust 工具链")

    if result.returncode != 0:
        stderr = result.stderr.decode("utf-8", "replace").strip()
        raise RustError(f"rustc {' '.join(args)} 失败: {stderr}")

    return result


def get_host_triple() -> str:
    """本机三元组"""
    for line in _run(["-vV"]).stdout.decode("utf-8", "replace").splitlines():
        key, sep, value = line.partition(":")
        if sep and key.strip() == "host":
            return value.strip()
    raise RustError("rustc -vV 输出中未找到 host 行")


@cache
def get_triples() -> list[str]:
    """rustc 支持的全部目标三元组"""
    output = _run(["--print", "target-list"]).stdout.decode("utf-8", "replace")
    return output.split()
