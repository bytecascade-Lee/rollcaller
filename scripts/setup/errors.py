#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
环境准备异常
"""

from cli import cargo, git, just, node, pnpm, rustc, uv


class EnvError(Exception):
    """工具链缺失或版本过低，或某个准备步骤执行失败"""


# 可能从 cli 门面传递上的底层失败
CLI_ERRORS: tuple[type[Exception], ...] = (
    cargo.CargoError,
    git.GitError,
    just.JustError,
    node.NodeError,
    pnpm.PnpmError,
    rustc.RustcError,
    uv.UvError,
)
