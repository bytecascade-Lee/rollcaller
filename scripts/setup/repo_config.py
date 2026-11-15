#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
仓库级 git 配置

只管与远端无关的固定键；远端名、拉取源与推送目标归 `remotes.py`。
"""

from cli import git
from common.logger import info

from .expectation import ADVICE, Expectation


def _git_config(key: str, expected: str | None, advice: str = ADVICE) -> Expectation[str]:
    """一条仓库级配置检查：对象名由键拼出来，expected 为 None 表示这个键不该存在"""
    return Expectation(key, f"git 配置 {key}", expected, advice=advice)


# 强制写在仓库级的配置: 只影响本仓库，不动用户的全局设置
LOCAL_GIT_CONFIG: tuple[Expectation[str], ...] = (
    # 拉取只允许快进: 分叉时报错，由人显式决定合并
    _git_config("pull.ff", "only"),
    # 长期分支上的合并必须留记录。其余分支之间合并仍可快进
    _git_config("branch.develop.mergeOptions", "--no-ff"),
    _git_config("branch.master.mergeOptions", "--no-ff"),
    # 中文路径不转义为八进制串
    _git_config("core.quotepath", "false"),
    # 远程已删除的分支在 fetch 时一并清理本地跟踪引用
    _git_config("fetch.prune", "true"),
    # 纯格式化提交的 hash 记在仓库根的 `.git-blame-ignore-revs`，blame 跳过它们
    _git_config("blame.ignoreRevsFile", ".git-blame-ignore-revs"),
    # 让被忽略的行显式标注出来，而不是伪装成"没有作者的本地改动"
    _git_config("blame.markIgnoredLines", "true"),
    _git_config("blame.markUnblamableLines", "true"),
)

# 旧策略遗留、必须从仓库级移除的键
LEGACY_GIT_CONFIG: tuple[Expectation[str], ...] = (_git_config("merge.ff", None, advice="执行 `just setup` 清理"),)


def problems() -> list[str]:
    """仓库级配置是否已按 LOCAL_GIT_CONFIG 写入、遗留键是否已移除"""
    result: list[str] = []
    for expectation in (*LOCAL_GIT_CONFIG, *LEGACY_GIT_CONFIG):
        actual = git.get_config(expectation.key)
        problem = expectation.judge(actual)
        if problem is not None:
            result.append(problem)
        elif actual is not None:
            # 期望"不该存在"的键通过时没有值可报，也就没有结论行
            info(f"{expectation.label}={actual}")
    return result


def apply() -> None:
    """逐条写入仓库级配置、移除遗留键；重复执行是幂等的"""
    for expectation in LOCAL_GIT_CONFIG:
        git.set_config(expectation.key, str(expectation.expected))
        info(f"{expectation.label}={expectation.expected}")

    for expectation in LEGACY_GIT_CONFIG:
        if git.get_config(expectation.key) is None:
            continue
        git.unset_config(expectation.key)
        info(f"{expectation.label} 已移除，约束收窄到 develop/master 的 mergeOptions")
