#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
cargo 命令封装
"""

from cli.tool import Timeout, Tool
from common.constant.paths import BACKEND_DIR
from common.entity.target import Target

CLIPPY_ALL_TARGETS = "--all-targets"
CLIPPY_DENY = "clippy::all"


class CargoError(Exception):
    """cargo 调用失败"""


# 默认按普通操作计时，fetch 走网络档，test / lint 走长任务档
# 默认不捕获，输出直接进控制台
TOOL = Tool(
    "cargo",
    CargoError,
    cwd=BACKEND_DIR,
    timeout=Timeout.NORMAL,
    capture=False,
    hint="请安装 Rust 工具链并加入 PATH：https://rustup.rs/",
)


def lock() -> None:
    """刷新工作区成员在 Cargo.lock 里的版本：`cargo update --workspace --offline`"""
    args = ["update", "--workspace", "--offline"]
    _ = TOOL.run(args)


def fetch() -> None:
    """下载后端依赖：`cargo fetch --locked`"""
    args = ["fetch", "--locked"]
    _ = TOOL.run(args, timeout=Timeout.NETWORK)


def test(target: Target, *, lib_only: bool = False) -> None:
    """
    运行后端测试：`cargo test --locked --target <triple> [--lib]`

    :param target: 测试的编译目标，产物落在 target/<triple> 下
    :param lib_only: 只运行 lib 单测，供 ts-rs 导出类型时使用
    """
    args = ["test", "--locked", "--target", target.triple, *(["--lib"] if lib_only else [])]
    _ = TOOL.run(args, timeout=Timeout.EXTENDED)


def lint(target: Target) -> None:
    """
    后端 lint：`cargo clippy --locked --target <triple> --all-targets -- -D clippy::all`

    :param target: lint 的编译目标

    :raise CargoError: clippy 组件缺失时给出安装提示
    """
    args = ["clippy", "--locked", "--target", target.triple, CLIPPY_ALL_TARGETS, "--", "-D", CLIPPY_DENY]
    try:
        _ = TOOL.run(args, timeout=Timeout.EXTENDED)
    except CargoError as error:
        if "no such command" in str(error) or "not installed" in str(error):
            raise CargoError("未找到 'cargo-clippy'，请执行 'rustup component add clippy' 安装") from error
        raise


def format(*, check: bool = False) -> None:
    """
    格式化后端：`cargo fmt [--check]`

    :param check: True 时只检查不写文件
    """
    args = ["fmt", *(["--check"] if check else [])]
    _ = TOOL.run(args)


def licenses(target: Target) -> str:
    """
    导出依赖元数据，JSON 格式：`cargo metadata --format-version 1 --offline --locked --filter-platform <triple>`

    :param target: 只保留目标平台需要的依赖
    """
    args = ["metadata", "--format-version", "1", "--offline", "--locked", "--filter-platform", target.triple]
    return TOOL.text(args)
