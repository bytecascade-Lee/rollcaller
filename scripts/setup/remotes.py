#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
远端拓扑
"""

from dataclasses import dataclass
from typing import Mapping

from cli import git
from common.constant.repo import GITHUB_REPO, LONG_LIVED_BRANCHES, MIRROR, REMOTE
from common.enums.remote import RemoteRole
from common.logger import info, success, warning
from common.remote import is_github, owner_repo, role_of
from .errors import EnvError

# fork 的约定名
FORK_REMOTE = "origin"

PUSH_DEFAULT = "remote.pushDefault"


@dataclass(frozen=True)
class Solution:
    """一份施工计划"""

    renames: tuple[tuple[str, str], ...] = ()
    additions: tuple[tuple[str, str], ...] = ()
    # (配置键, 期望值, 当前值, 远端改名后的值)：期望值为 None 表示这个键不该存在；
    # 当前值与改名后值不同，说明这条是"rename 顺手改写了配置"造成的
    configs: tuple[tuple[str, str | None, str | None, str | None], ...] = ()
    blockers: tuple[str, ...] = ()
    notes: tuple[str, ...] = ()
    push: str = REMOTE

    @property
    def pending(self) -> bool:
        return bool(self.renames or self.additions or self.configs)


@dataclass(frozen=True)
class Observed:
    """`_observe()` 采到的现状；`_solve()` 只吃这个，因此可以脱开 git 单测"""

    remotes: Mapping[str, str]
    # 已存在的长期分支，只对它们设置上游
    branches: tuple[str, ...]
    # 被管理的配置键及其当前值，键集合由 branches 与 PUSH_DEFAULT 决定
    config: Mapping[str, str | None]


def problems() -> list[str]:
    """拉取源、推送目标、长期分支的上游与分叉推送配置是否就位"""
    solution = _solve(_observe())
    if solution.blockers:
        return list(solution.blockers)
    for note in solution.notes:
        warning(note)
    if solution.pending:
        return [f"{item}；执行 `just setup`" for item in _describe(solution)]
    _report(solution)
    return []


def apply() -> None:
    """先算完整计划再动手：有 blocker 时一条都不执行，不留改一半的仓库"""
    solution = _solve(_observe())
    if solution.blockers:
        raise EnvError("远端拓扑需人工处理: " + "；".join(solution.blockers))

    for old, new in solution.renames:
        git.rename_remote(old, new)
        success(f"远端 {old} 已改名为 {new}")
    for name, url in solution.additions:
        git.add_remote(name, url)
        success(f"已新增远端 {name} → {url}")
    for key, value, *_ in solution.configs:
        if value is None:
            git.unset_config(key)
            info(f"git 配置 {key} 已删除（不再是分叉工作流）")
        else:
            git.set_config(key, value)
            info(f"git 配置 {key}={value}")

    for note in solution.notes:
        warning(note)
    _report(solution)


def _observe() -> Observed:
    """采集远端、长期分支与相关配置；`git remote` 与 `remote get-url` 同源，取不到 URL 时记空串"""
    remotes = {name: git.remote_url(name) or "" for name in git.remotes()}
    branches = tuple(name for name in LONG_LIVED_BRANCHES if git.try_rev_parse(f"refs/heads/{name}") is not None)
    keys = [f"branch.{branch}.{field}" for branch in branches for field in ("remote", "merge")]
    config = {key: git.get_config(key) for key in (*keys, PUSH_DEFAULT)}
    return Observed(remotes=remotes, branches=branches, config=config)


def _solve(observed: Observed) -> Solution:
    """由现状推出目标拓扑；纯函数，不执行任何 git 命令"""
    if not observed.remotes:
        return Solution(notes=("仓库没有配置任何远端，跳过远端归一",))

    # 边算边维护一份"改名后"的视图，后续判断才不会踩到已改过的名字
    state = dict(observed.remotes)
    renames: list[tuple[str, str]] = []
    additions: list[tuple[str, str]] = []
    notes: list[str] = []
    blockers: list[str] = []

    def rename(old: str, new: str) -> None:
        if new in state:
            blockers.append(f"无法把远端 {old} 改名为 {new}：该名字已被 {state[new]!r} 占用，请手工处理")
            return
        renames.append((old, new))
        state[new] = state.pop(old)

    # 一、`github` 归本项目所有：被别的地址占着就按它的真实角色让位
    if REMOTE in state:
        occupant = role_of(state[REMOTE])
        if occupant is RemoteRole.MIRROR:
            rename(REMOTE, MIRROR)
        elif occupant is not RemoteRole.CANONICAL:
            if occupant is RemoteRole.FORK or is_github(state[REMOTE]):
                rename(REMOTE, FORK_REMOTE)
            else:
                blockers.append(f"远端 {REMOTE} 指向与本仓库无关的 {state[REMOTE]!r}，请手工改名后重试")

    # 二、规范仓库落到 github 名下：优先改名已有远端，其次新增（不 fetch）
    if REMOTE not in state:
        canonical = [name for name, url in state.items() if role_of(url) is RemoteRole.CANONICAL]
        if canonical:
            rename(canonical[0], REMOTE)
        else:
            url = _canonical_url(observed.remotes)
            additions.append((REMOTE, url))
            state[REMOTE] = url

    # 三、镜像统一叫 cnb；这是软约定，名字被占用就保留原名并提示
    mirrors = [name for name, url in state.items() if role_of(url) is RemoteRole.MIRROR]
    if mirrors:
        promoted = MIRROR if MIRROR in mirrors else mirrors[0]
        if promoted != MIRROR:
            if MIRROR in state:
                notes.append(f"远端 {promoted} 是 CNB 镜像，但名字 {MIRROR} 已被占用，保持原名")
            else:
                rename(promoted, MIRROR)
        for name in mirrors:
            if name not in (promoted, MIRROR):
                notes.append(f"远端 {name} 同样指向 CNB 镜像，未改动")
    elif MIRROR in state:
        notes.append(f"远端 {MIRROR} 指向的不是 CNB 镜像（{state[MIRROR]}），未改动")

    # 四、推送目标：有 fork 就推 fork，否则推规范仓库
    forks = [name for name, url in state.items() if role_of(url) is RemoteRole.FORK]
    # 判断不了是不是同一个仓库的 GitHub 远端（例如 fork 改过名）也算 fork，但要说清楚
    loose = [name for name, url in state.items() if role_of(url) is RemoteRole.UNRELATED and is_github(url)]
    for name in loose:
        notes.append(f"远端 {name} 指向 GitHub 上的 {owner_repo(state[name])}，已按你的 fork 处理；"
                     f"若它不是，请 `git remote rename {name} <名字>`")

    candidates = forks + loose
    push = REMOTE
    if candidates:
        push = FORK_REMOTE if FORK_REMOTE in candidates else sorted(candidates)[0]
        if push != FORK_REMOTE:
            if FORK_REMOTE in state:
                notes.append(f"远端 {push} 是你的 fork，但名字 {FORK_REMOTE} 已被占用，推送目标保持 {push}")
            else:
                rename(push, FORK_REMOTE)
                push = FORK_REMOTE
        if len(candidates) > 1:
            notes.append(f"有多个 GitHub 远端（{', '.join(sorted(candidates))}），推送目标取 {push}")

    return Solution(
        renames=tuple(renames),
        additions=tuple(additions),
        configs=tuple(_config_changes(observed, push, dict(renames))),
        blockers=tuple(blockers),
        notes=tuple(notes),
        push=push,
    )


def _config_changes(observed: Observed, push: str, renames: Mapping[str, str],
                    ) -> list[tuple[str, str | None, str | None, str | None]]:
    """长期分支的上游一律指向 github；`remote.pushDefault` 只在分叉工作流时存在"""
    changes: list[tuple[str, str | None, str | None, str | None]] = []
    for branch in observed.branches:
        upstream_key = f"branch.{branch}.remote"
        merge_key = f"branch.{branch}.merge"
        upstream = observed.config.get(upstream_key)
        # `git remote rename` 会顺手改写 branch.<b>.remote，按计划投影后再比较，避免重复报同一件事
        projected = renames.get(upstream or "", upstream)
        if projected != REMOTE:
            changes.append((upstream_key, REMOTE, upstream, projected))
        merge = observed.config.get(merge_key)
        if merge != f"refs/heads/{branch}":
            changes.append((merge_key, f"refs/heads/{branch}", merge, merge))

    push_default = observed.config.get(PUSH_DEFAULT)
    if push == REMOTE:
        # 单远端时保持 git 默认：让临时分支继续跟随自己的 -u，而不是被全局推送目标改写
        if push_default is not None:
            changes.append((PUSH_DEFAULT, None, push_default, push_default))
    elif push_default != push:
        changes.append((PUSH_DEFAULT, push, push_default, push_default))
    return changes


def _canonical_url(remotes: Mapping[str, str]) -> str:
    """规范仓库 URL"""
    for url in remotes.values():
        if not is_github(url):
            continue
        text = url.strip()
        if text.casefold().startswith("ssh://"):
            return f"ssh://git@github.com/{GITHUB_REPO}.git"
        if "://" not in text and "@" in text:
            return f"git@github.com:{GITHUB_REPO}.git"
        return f"https://github.com/{GITHUB_REPO}.git"
    return f"https://github.com/{GITHUB_REPO}.git"


def _describe(solution: Solution) -> list[str]:
    """把待施工项翻成 doctor 的一句话"""
    items = [f"远端 {old} 应改名为 {new}" for old, new in solution.renames]
    items += [f"远端 {name} 缺失，应新增 {name} → {url}" for name, url in solution.additions]
    for key, value, current, projected in solution.configs:
        expected = "未设置" if value is None else repr(value)
        if projected == current:
            items.append(f"git 配置 {key} 期望 {expected}，实际 {current!r}")
        else:
            items.append(f"git 配置 {key} 期望 {expected}，随远端改名将变成 {projected!r}")
    return items


def _report(solution: Solution) -> None:
    """体检通过或施工完成后的结论行"""
    info(f"拉取源 {REMOTE} → {GITHUB_REPO}")
    if solution.push == REMOTE:
        info(f"推送目标 {solution.push}（规范仓库）")
    else:
        info(f"推送目标 {solution.push}（这是你的 fork，PR 从这里提）")
