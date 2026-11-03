#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
cargo 命令封装
"""

import subprocess
from pathlib import Path
from typing import Sequence

from cli import _process
from cli._version import version_string
from common.constant.paths import BACKEND_DIR
from common.entity.target import Target

CLIPPY_ALL_TARGETS = "--all-targets"
CLIPPY_DENY = "clippy::all"


class CargoError(Exception):
    """cargo 调用失败"""


def _run(args: Sequence[str], *, cwd: Path | None = None, capture: bool = False) -> subprocess.CompletedProcess:
    """执行 cargo 命令，失败抛 CargoError；默认不捕获，构建输出直接进控制台"""
    cwd = cwd or BACKEND_DIR
    try:
        return _process.run(["cargo", *args], cwd=cwd, capture=capture)
    except _process.ProcessError as error:
        if error.not_found:
            raise CargoError("未找到 'cargo'，请确保已安装 Rust 工具链并加入 PATH") from error
        raise CargoError(str(error)) from error


def cargo(args: Sequence[str], cwd: Path | None = None, capture: bool = False) -> str:
    """执行 cargo 命令；capture=True 返回 strip 后的 stdout，否则直接输出到控制台"""
    result = _run(args, cwd=cwd, capture=capture)
    return str(result.stdout).strip() if capture else ""


def fetch(cwd: Path | None = None) -> None:
    """下载后端依赖：`cargo fetch --locked`"""
    cargo(["fetch", "--locked"], cwd=cwd)


def test(target: Target, *, lib_only: bool = False, cwd: Path | None = None) -> None:
    """
    运行后端测试：`cargo test --locked --target <triple> [--lib]`

    :param target: 测试的编译目标，产物落在 target/<triple> 下
    :param lib_only: 只运行 lib 单测，供 ts-rs 导出类型时使用
    :param cwd: 工作目录，缺省后端目录
    """
    cargo(["test", "--locked", "--target", target.triple, *(["--lib"] if lib_only else [])], cwd=cwd)


def check(target: Target, *, lib: bool = False, cwd: Path | None = None) -> None:
    """
    只编译不链接：`cargo check --locked --target <triple> [--lib]`

    :param target: 编译的编译目标
    :param lib: 只检查 lib，不处理 bin / test / example
    :param cwd: 工作目录，缺省后端目录
    """
    cargo(["check", "--locked", "--target", target.triple, *(["--lib"] if lib else [])], cwd=cwd)


def clippy(target: Target, *, cwd: Path | None = None) -> None:
    """
    后端 lint：`cargo clippy --locked --target <triple> --all-targets -- -D clippy::all`

    :param target: lint 的编译目标
    :param cwd: 工作目录，缺省后端目录

    :raise CargoError: clippy 组件缺失时给出安装提示
    """
    args = ["clippy", "--locked", "--target", target.triple, CLIPPY_ALL_TARGETS, "--", "-D", CLIPPY_DENY]
    try:
        cargo(args, cwd=cwd)
    except CargoError as error:
        if "no such command" in str(error) or "not installed" in str(error):
            raise CargoError("未找到 'cargo-clippy'，请执行 'rustup component add clippy' 安装") from error
        raise


def fmt_check(cwd: Path | None = None) -> None:
    """检查格式：`cargo fmt --check`"""
    cargo(["fmt", "--check"], cwd=cwd)


def version() -> str | None:
    """返回 `cargo --version` 里的版本号；否则返回 None"""
    return version_string(cargo(["--version"], capture=True))
