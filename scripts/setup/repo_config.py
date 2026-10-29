#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
仓库级 git 配置

只管与远端无关的固定键；远端名、拉取源与推送目标归 `remotes.py`。
"""

from cli import git
from common.logger import info

# 强制写在仓库级的配置: 只影响本仓库，不动用户的全局设置
LOCAL_GIT_CONFIG: tuple[tuple[str, str], ...] = (
    # 拉取只允许快进: 分叉时报错，由人显式决定合并
    ("pull.ff", "only"),
    # 长期分支上的合并必须留记录。其余分支之间合并仍可快进
    ("branch.develop.mergeOptions", "--no-ff"),
    ("branch.master.mergeOptions", "--no-ff"),
    # 中文路径不转义为八进制串
    ("core.quotepath", "false"),
    # 远程已删除的分支在 fetch 时一并清理本地跟踪引用
    ("fetch.prune", "true"),
)

# 旧策略遗留、必须从仓库级移除的键
LEGACY_GIT_CONFIG: tuple[str, ...] = (
    "merge.ff",
)


def problems() -> list[str]:
    """仓库级配置是否已按 LOCAL_GIT_CONFIG 写入、遗留键是否已移除"""
    result: list[str] = []
    for key, expected in LOCAL_GIT_CONFIG:
        actual = git.get_config(key)
        if actual == expected:
            info(f"git 配置 {key}={actual}")
            continue
        result.append(f"git 配置 {key} 期望 {expected!r}，实际情况 {actual!r}；执行 `just setup` 写入")

    for key in LEGACY_GIT_CONFIG:
        actual = git.get_config(key)
        if actual is not None:
            result.append(
                f"git 配置 {key}={actual!r} 是全局旧策略，应在仓库级移除；执行 `just setup` 清理"
            )
    return result


def apply() -> None:
    """逐条写入仓库级配置、移除遗留键；重复执行是幂等的"""
    for key, value in LOCAL_GIT_CONFIG:
        git.set_config(key, value)
        info(f"git 配置 {key}={value}")

    for key in LEGACY_GIT_CONFIG:
        if git.get_config(key) is None:
            continue
        git.unset_config(key)
        info(f"git 配置 {key} 已移除，约束收窄到 develop/master 的 mergeOptions")
