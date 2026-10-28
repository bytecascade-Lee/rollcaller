#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
环境准备异常
"""

from cli import git, just, node, pnpm, rust, uv


class EnvError(Exception):
    """工具链缺失或版本过低，或某个准备步骤执行失败"""


# 可能从 cli 门面传递上的底层失败
CLI_ERRORS: tuple[type[Exception], ...] = (
    git.GitError,
    just.JustError,
    node.NodeError,
    pnpm.PnpmError,
    rust.RustError,
    uv.UvError,
)
