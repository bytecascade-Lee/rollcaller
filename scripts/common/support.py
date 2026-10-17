#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
支持目标注册表
"""

from common.entity.target import SupportedTarget, Target, UnsupportedTargetError
from common.enums.action import Action
from common.enums.kind import Kind


class SupportTableError(Exception):
    """支持表自身的注册错误（重复、空集合、运行器与目标 OS 不匹配等）"""


SUPPORTED_TARGETS: tuple[SupportedTarget, ...] = (
    SupportedTarget(
        target=Target("x86_64-pc-windows-msvc"),
        actions=frozenset({Action.BUILD, Action.RELEASE}),
        kinds=frozenset({Kind.NSIS, Kind.PORTABLE, Kind.DEVELOP}),
        runner="windows-latest",
    ),
    SupportedTarget(
        target=Target("aarch64-pc-windows-msvc"),
        actions=frozenset({Action.BUILD, Action.RELEASE}),
        kinds=frozenset({Kind.NSIS, Kind.PORTABLE, Kind.DEVELOP}),
        runner="windows-latest",
    ),
)

_BY_TRIPLE: dict[str, SupportedTarget] = {
    entry.target.triple: entry for entry in SUPPORTED_TARGETS
}

# 各模式期望产出的载荷；target 自身不支持某个 kind 时，取交集后自然忽略
RELEASE_KINDS: frozenset[Kind] = frozenset({Kind.NSIS, Kind.PORTABLE})
LOCAL_KINDS: frozenset[Kind] = RELEASE_KINDS | {Kind.DEVELOP}


def validate() -> None:
    """导入期自检：任何一条不成立都说明注册表写错了"""
    seen: set[str] = set()
    for entry in SUPPORTED_TARGETS:
        triple = entry.target.triple
        if triple in seen:
            raise SupportTableError(f"目标三元组重复注册: {triple!r}")
        seen.add(triple)
        if not entry.actions:
            raise SupportTableError(f"{triple!r} 未声明任何 Action")
        if not entry.kinds:
            raise SupportTableError(f"{triple!r} 未声明任何 Kind")
        if not entry.runner.strip():
            raise SupportTableError(f"{triple!r} 未声明运行器")


def get(target: Target) -> SupportedTarget | None:
    """按 Target 或三元组字符串查找，未注册返回 None"""
    return _BY_TRIPLE.get(target.triple)


def require(target: Target, action: Action) -> SupportedTarget:
    """要求目标开放指定动作档位，未注册与未开放分别抛错"""
    entry = get(target)
    if entry is None:
        raise UnsupportedTargetError(f"目标三元组 {target.triple!r} 未在支持表中注册")
    entry.require(action)
    return entry


def entries(action: Action) -> tuple[SupportedTarget, ...]:
    """开放指定动作档位的全部注册项，保持声明顺序，矩阵按此生成"""
    return tuple(entry for entry in SUPPORTED_TARGETS if entry.supports(action))


def targets(action: Action) -> tuple[Target, ...]:
    """开放指定动作档位的目标三元组，保持声明顺序"""
    return tuple(entry.target for entry in entries(action))


def kinds(target: Target) -> frozenset[Kind]:
    """该目标可产出的全部载荷，未注册抛 UnsupportedTargetError"""
    entry = get(target)
    if entry is None:
        raise UnsupportedTargetError(f"目标三元组 {target!r} 未在支持表中注册")
    return entry.kinds


validate()
