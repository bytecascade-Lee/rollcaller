#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
ts-rs 类型导出
"""

from cli import cargo
from common.constant.paths import CARGO_CONFIG_PATH, PROJECT_BASE_DIR


class TSRSError(Exception):
    """pass"""


def export() -> None:
    """跑 lib 测试，触发 ts-rs 把类型写回 `frontend/src/lib/types`"""
    if not CARGO_CONFIG_PATH.is_file():
        relative = CARGO_CONFIG_PATH.relative_to(PROJECT_BASE_DIR).as_posix()
        raise TSRSError(f"缺少 {relative}，ts-rs 会导出到默认目录，无法校验类型；请先跑 just setup")

    cargo.test(["--locked", "--lib"])
