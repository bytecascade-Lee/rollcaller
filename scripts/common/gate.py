#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
入口闸门

在这台机器、这个档位下，现在能不能做：

    1. 目标必须已在支持表中注册
    2. 目标必须开放本次要用的档位
    3. 目标与宿主的 OS 必须一致
"""

from common import support
from common.entity.target import SupportedTarget, Target
from common.enums.action import Action
from util.str_format import join_repr


class GateError(Exception):
    """目标在当前宿主 / 当前档位下不可构建"""


def ensure(target: Target, action: Action, *, host: Target) -> SupportedTarget:
    """
    校验目标可在宿主上执行指定档位，通过则返回支持表条目

    :param target: 待校验的构建目标
    :param action: 本次要做的事，决定档位门槛
    :param host:   宿主目标，跨系统判定只比较两者的 OS
    """
    entry = support.get(target)
    if entry is None:
        known = join_repr(item.target.triple for item in support.entries(Action.BUILD))
        raise GateError(f"目标三元组 {target.triple!r} 未在支持表中注册；当前可构建: {known}")

    if not entry.supports(action):
        opened = join_repr(sorted(item.value for item in entry.actions))
        raise GateError(f"目标 {target.triple!r} 未开放 {action.value!r} 档位（已开放: {opened}）")

    if entry.target.os != host.os:
        raise GateError(
            f"不支持跨系统构建：宿主 {host.triple!r} 的 OS 为 {host.os.value!r}，无法为 "
            f"{target.triple!r} 构建；请改用对应系统的机器或 CI runner"
        )

    return entry
