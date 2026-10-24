#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""rustc 命令封装"""

import subprocess
from functools import cache
from pathlib import Path

from cli import process
from common.constant.paths import BACKEND_DIR, PROJECT_BASE_DIR
from common.entity.target import Target, UnsupportedTargetError


class RustError(Exception):
    """rustc 调用失败"""
    pass


def _run(args: list[str], cwd: Path | None = None, *, capture: bool = True) -> subprocess.CompletedProcess:
    """执行 rust 命令，失败抛 RustError"""
    cwd = cwd or PROJECT_BASE_DIR
    try:
        return process.run(args, cwd=cwd, capture=capture)
    except process.ProcessError as error:
        if error.not_found:
            raise RustError(f"未找到 {args[0]!r}，请确保已安装 Rust 工具链并加入 PATH") from error
        raise RustError(str(error)) from error


def rustc(args: list[str], cwd: Path | None = None) -> str:
    return str(_run(["rustc", *args], cwd).stdout).strip()


def cargo(args: list[str], cwd: Path | None = None) -> str:
    return str(_run(["cargo", *args], cwd).stdout).strip()


def get_host_target() -> Target:
    """本机三元组"""
    for line in rustc(["-vV"]).splitlines():
        if line.startswith("host:"):
            return Target(line.removeprefix("host:").strip())
    raise RustError("rustc -vV 输出中未找到 host 行")


@cache
def get_triples() -> list[str]:
    """rustc 支持的全部目标三元组"""
    return rustc(["--print", "target-list"]).splitlines()


def is_supported(triple: str) -> bool:
    return triple in get_triples()


def resolve_triple(triple: str) -> str:
    if not is_supported(triple):
        raise UnsupportedTargetError(f"不是有效的 Rust 目标三元组: {triple!r}")
    return triple


def fetch(cwd: Path | None = None) -> None:
    """执行 `cargo fetch --locked`，`--locked` 不改写 Cargo.lock；锁文件与 Cargo.toml 不同步时直接失败"""
    cargo(["fetch", "--locked"], cwd or BACKEND_DIR)
