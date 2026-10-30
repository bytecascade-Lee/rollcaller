#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
代码生成包的统一入口
"""

from collections.abc import Sequence
from pathlib import Path
from types import ModuleType

from cli import git
from common.constant.paths import FRONTEND_ROUTING_ENTRIES_PATH
from common.logger import info
from util.str_format import join_repr
from . import build_info, routing_entries


class CodegenError(Exception):
    """代码生成阶段失败"""


class GeneratedNotCommittedError(Exception):
    """生成物相对 HEAD 有改动：CI 的生成物校验会失败"""


# 顺序即执行顺序
GENERATORS: tuple[ModuleType, ...] = (build_info, routing_entries)

# 受版本控制的生成物；build_info.json 不参与校验
GENERATED_PATHS: tuple[Path, ...] = (FRONTEND_ROUTING_ENTRIES_PATH,)


def generate_all() -> None:
    """
    串行执行全部生成器，任一失败立即中止

    :raise CodegenError: 某个生成器抛异常，消息形如 `generate_xxx 失败: <原因>`
    """
    for module in GENERATORS:
        name = module.__name__.removeprefix("codegen.")
        info(f"代码生成: {name}")
        try:
            module.generate()
        except Exception as error:
            raise CodegenError(f"{name} 失败: {error}") from error
    info(f"代码生成完成")


def ensure_committed(paths: Sequence[Path] = GENERATED_PATHS, *, label: str = "生成物") -> None:
    """生成物必须与 HEAD 一致"""
    names = git.diff_names(paths)
    if not names:
        return
    raise GeneratedNotCommittedError(
        f"{label} 有 {len(names)} 个文件未提交: {join_repr(names)}；先提交再继续"
    )
