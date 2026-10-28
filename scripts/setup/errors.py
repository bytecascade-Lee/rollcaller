#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
环境准备的统一错误
"""


# 单独成一个叶子模块，若定义在包 `__init__` 里，子模块 import 时
# 包还没初始化完，就只能靠"定义写在 import 之前"这种隐性顺序保证。

class EnvError(Exception):
    """工具链缺失或版本过低，或某个准备步骤执行失败"""
