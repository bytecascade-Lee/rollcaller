#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""rustc 命令封装"""

import subprocess
from functools import cache
from pathlib import Path
from typing import List, Optional

from common.entity.target import Target, UnsupportedTargetError
from common.logger import debug


class RustError(Exception):
    """rustc 调用失败"""
    pass


def _run(args: List[str], cwd: Optional[Path] = None) -> subprocess.CompletedProcess:
    """执行 rust 命令，失败抛 RustError"""
    debug(f"执行: {' '.join(args)}")
    try:
        result = subprocess.run(
            args,
            cwd=cwd,
            capture_output=True,
            check=False,
        )
    except FileNotFoundError:
        raise RustError("未找到 rust 命令，请确保已安装 Rust 工具链")

    if result.returncode != 0:
        stderr = result.stderr.decode("utf-8", "replace").strip()
        raise RustError(f"命令 {' '.join(args)} 失败: {stderr}")

    return result


def rustc(args: List[str], cwd: Optional[Path] = None) -> str:
    return _run(["rustc", *args], cwd).stdout.decode("utf-8", "replace").strip()


def get_host_target() -> Target:
    """本机三元组"""
    for line in rustc(["-vV"]).splitlines():
        if line.startswith("host:"):
            return Target(line.removeprefix("host:").strip())
    raise RustError("rustc -vV 输出中未找到 host 行")


@cache
def get_triples() -> List[str]:
    """rustc 支持的全部目标三元组"""
    return rustc(["--print", "target-list"]).splitlines()


def is_supported(triple: str) -> bool:
    return triple in get_triples


def resolve_triple(triple: Optional[str]) -> str:
    if triple is None:
        return get_host_target().triple
    if not is_supported(triple):
        raise UnsupportedTargetError(f"不是有效的 Rust 目标三元组: {triple!r}")
    return triple
