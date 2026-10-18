#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
代码生成包的统一入口
"""

from types import ModuleType

from common.logger import info
from . import generate_build_info, generate_routing_entries


class CodegenError(Exception):
    """代码生成阶段失败"""


# 顺序即执行顺序
GENERATORS: tuple[ModuleType, ...] = (generate_build_info, generate_routing_entries)


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
