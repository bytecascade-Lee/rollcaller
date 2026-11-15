#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
rustc 命令封装
"""

from functools import cache

from cli._tool import Tool
from common.constant.paths import BACKEND_DIR
from common.entity.target import Target, UnsupportedTargetError


class RustcError(Exception):
    """rustc 调用失败"""


TOOL = Tool(
    "rustc",
    RustcError,
    cwd=BACKEND_DIR,
    hint="请安装 Rust 工具链并加入 PATH：https://rustup.rs/",
)


def host_target() -> Target:
    """本机三元组"""
    for line in TOOL.text(["-vV"]).splitlines():
        if line.startswith("host:"):
            return Target(line.removeprefix("host:").strip())
    raise RustcError("rustc -vV 输出中未找到 host 行")


@cache
def triples() -> list[str]:
    """rustc 支持的全部目标三元组"""
    return TOOL.text(["--print", "target-list"]).splitlines()


def is_supported(triple: str) -> bool:
    return triple in triples()


def resolve_triple(triple: str) -> str:
    if not is_supported(triple):
        raise UnsupportedTargetError(f"不是有效的 Rust 目标三元组: {triple!r}")
    return triple
