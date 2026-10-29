#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
cargo 命令封装
"""

import subprocess
from pathlib import Path

from cli import _process
from cli._version import version_string
from common.constant.paths import BACKEND_DIR


class CargoError(Exception):
    """cargo 调用失败"""


def _run(args: list[str], *, cwd: Path | None = None, capture: bool = False) -> subprocess.CompletedProcess:
    """执行 cargo 命令，失败抛 CargoError；默认不捕获，构建输出直接进控制台"""
    cwd = cwd or BACKEND_DIR
    try:
        return _process.run(["cargo", *args], cwd=cwd, capture=capture)
    except _process.ProcessError as error:
        if error.not_found:
            raise CargoError("未找到 'cargo'，请确保已安装 Rust 工具链并加入 PATH") from error
        raise CargoError(str(error)) from error


def cargo(args: list[str], cwd: Path | None = None, capture: bool = False) -> str:
    """执行 cargo 命令；capture=True 返回 strip 后的 stdout，否则直接输出到控制台"""
    result = _run(args, cwd=cwd, capture=capture)
    return str(result.stdout).strip() if capture else ""


def fetch(cwd: Path | None = None) -> None:
    """执行 `cargo fetch --locked`"""
    cargo(["fetch", "--locked"], cwd=cwd)


def test(args: list[str], cwd: Path | None = None) -> None:
    """执行 `cargo test`"""
    cargo(["test", *args], cwd=cwd)


def version() -> str | None:
    """返回 `cargo --version` 里的版本号；否则返回 None"""
    return version_string(cargo(["--version"], capture=True))
