#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
rustc 命令封装
"""

import subprocess
from functools import cache
from pathlib import Path

from cli import _process
from cli._version import version_string
from common.constant.paths import BACKEND_DIR
from common.entity.target import Target, UnsupportedTargetError


class RustcError(Exception):
    """rustc 调用失败"""


def _run(args: list[str], *, cwd: Path | None = None) -> subprocess.CompletedProcess:
    """执行 rustc 命令，失败抛 RustcError；rustc 的输出都要读，故恒捕获"""
    cwd = cwd or BACKEND_DIR
    try:
        return _process.run(["rustc", *args], cwd=cwd, capture=True)
    except _process.ProcessError as error:
        if error.not_found:
            raise RustcError("未找到 'rustc'，请确保已安装 Rust 工具链并加入 PATH") from error
        raise RustcError(str(error)) from error


def rustc(args: list[str], cwd: Path | None = None) -> str:
    """执行 rustc 命令，返回 strip 后的 stdout"""
    return str(_run(args, cwd=cwd).stdout).strip()


def host_target() -> Target:
    """本机三元组"""
    for line in rustc(["-vV"]).splitlines():
        if line.startswith("host:"):
            return Target(line.removeprefix("host:").strip())
    raise RustcError("rustc -vV 输出中未找到 host 行")


@cache
def triples() -> list[str]:
    """rustc 支持的全部目标三元组"""
    return rustc(["--print", "target-list"]).splitlines()


def is_supported(triple: str) -> bool:
    return triple in triples()


def resolve_triple(triple: str) -> str:
    if not is_supported(triple):
        raise UnsupportedTargetError(f"不是有效的 Rust 目标三元组: {triple!r}")
    return triple


def version() -> str | None:
    """`rustc --version` 里的版本号；输出认不出时返回 None"""
    return version_string(rustc(["--version"]))
