#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""rustc 命令封装"""

import subprocess
from functools import cache
from pathlib import Path
from typing import List, Optional

from cli import process
from common.entity.target import Target, UnsupportedTargetError


class RustError(Exception):
    """rustc 调用失败"""
    pass


def _run(args: List[str], cwd: Optional[Path] = None) -> subprocess.CompletedProcess:
    """执行 rust 命令，失败抛 RustError"""
    try:
        return process.run(args, cwd=cwd)
    except process.ProcessError as error:
        if error.not_found:
            raise RustError(f"未找到 {args[0]!r}，请确保已安装 Rust 工具链并加入 PATH") from error
        raise RustError(str(error)) from error


def rustc(args: List[str], cwd: Optional[Path] = None) -> str:
    return str(_run(["rustc", *args], cwd).stdout).strip()


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
    return triple in get_triples()


def resolve_triple(triple: str) -> str:
    if not is_supported(triple):
        raise UnsupportedTargetError(f"不是有效的 Rust 目标三元组: {triple!r}")
    return triple
