#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
格式化入口

顺序执行：
    前端格式 -> 后端格式 -> 脚本格式

写入式入口：只负责把代码改成符合各自格式权威（Prettier / rustfmt / ruff format）的样子。
只检查不写文件请用 check.py。

用法:
    uv run ./scripts/format.py
"""

import sys
from collections.abc import Callable

from cli import cargo, pnpm, ruff
from common.logger import error, info, success
from util.sigint import graceful_sigint, never_interrupted


def main(*, is_interrupted: Callable[[], bool] = never_interrupted) -> int:
    info("格式化前端")
    pnpm.format()
    if is_interrupted():
        return 0

    info("格式化后端")
    cargo.format()
    if is_interrupted():
        return 0

    info("格式化脚本")
    ruff.format()

    success("前端、后端与脚本格式化完成")
    return 0


if __name__ == "__main__":
    with graceful_sigint() as is_interrupted:
        try:
            sys.exit(main(is_interrupted=is_interrupted))
        except (cargo.CargoError, pnpm.PnpmError, ruff.RuffError) as e:
            if not is_interrupted():
                error(f"格式化失败: {e}")
                sys.exit(1)
