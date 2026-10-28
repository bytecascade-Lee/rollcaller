#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
仓库级 git 设置

两件事：把指向本仓库的远端统一命名为 `github`，并写入本仓库必须成立的那几条配置。
远端 URL 只比 host + owner/name，SSH 与 HTTPS 写法归一后再比。
"""

import re
from typing import Callable, TypeVar

from cli import git
from common.constant.repo import GITHUB_REPO, REMOTE
from common.logger import info, success, warning
from .errors import EnvError

_T = TypeVar("_T")

# 强制写在仓库级的配置: 只影响本仓库，不动用户的全局设置
LOCAL_GIT_CONFIG: tuple[tuple[str, str], ...] = (
    # 拉取只允许快进: 分叉时报错，由人显式决定合并，避免 pull 造出无意义的合并提交
    ("pull.ff", "only"),
    # 合并必须产生合并提交，不允许快进
    ("merge.ff", "false"),
    # 中文路径进行八进制转义
    ("core.quotepath", "false"),
    # 远程已删除的分支在 fetch 时一并清理本地跟踪引用
    ("fetch.prune", "true"),
)


def problems() -> list[str]:
    """远端是否已统一命名为 github 并指向本仓库，仓库级配置是否已按 LOCAL_GIT_CONFIG 写入"""
    return [*_remote_problems(), *_config_problems()]


def apply() -> None:
    """先归一远端再写配置；两条都是幂等的"""
    _ensure_remote()
    _apply_config()


def _call(action: str, function: Callable[..., _T], *args: str) -> _T:
    """调用 cli.git，失败统一转成 EnvError，错误前缀由调用方给出"""
    try:
        return function(*args)
    except git.GitError as error:
        raise EnvError(f"{action}失败: {error}") from error


def _remote_problems() -> list[str]:
    """检查远端是否已统一命名为 github 并指向本仓库"""
    remotes = _call("读取 git 远端", git.remotes)
    if not remotes:
        return []
    if REMOTE in remotes and _is_our_repo(_remote_url(REMOTE)):
        info(f"远端 {REMOTE} 指向 {GITHUB_REPO}")
        return []
    return [f"远端 {REMOTE} 未就位（当前远端: {', '.join(remotes)}）；执行 `just setup` 自动归一"]


def _config_problems() -> list[str]:
    """检查仓库级配置是否已按 LOCAL_GIT_CONFIG 写入"""
    result: list[str] = []
    for key, expected in LOCAL_GIT_CONFIG:
        actual = _call(f"读取 git 配置 {key}", git.get_config, key)
        if actual == expected:
            info(f"git 配置 {key}={actual}")
            continue
        result.append(f"git 配置 {key} 期望 {expected!r}，实际情况 {actual!r}；执行 `just setup` 写入")
    return result


def _ensure_remote() -> None:
    """把指向本仓库的远端统一改名为 `github`"""
    remotes = _call("读取 git 远端", git.remotes)
    if not remotes:
        warning("仓库没有配置任何远端，跳过远端归一")
        return

    current_url = _remote_url(REMOTE)
    if REMOTE in remotes and _is_our_repo(current_url):
        info(f"远端 {REMOTE} 已指向 {GITHUB_REPO}")
        return

    candidates = [name for name in remotes if name != REMOTE and _is_our_repo(_remote_url(name))]
    if not candidates:
        if REMOTE in remotes:
            raise EnvError(f"远端 {REMOTE} 指向的不是 {GITHUB_REPO}（当前 {current_url!r}），请自行确认")
        warning(f"没有远端指向 {GITHUB_REPO}（当前远端: {', '.join(remotes)}），跳过远端归一")
        return

    if REMOTE in remotes:
        raise EnvError(f"已存在远端 {REMOTE}（{current_url!r}），无法把 {candidates[0]!r} 改名过去")

    source = candidates[0]
    _call(f"重命名远端 {source}", git.rename_remote, source, REMOTE)
    success(f"远端 {source} 已重命名为 {REMOTE}")


def _apply_config() -> None:
    """逐条写入仓库级配置；同值重复写入是幂等的"""
    for key, value in LOCAL_GIT_CONFIG:
        _call(f"写入 git 配置 {key}", git.set_config, key, value)
        info(f"git 配置 {key}={value}")


def _remote_url(name: str) -> str | None:
    """远端地址；该远端不存在时返回 None"""
    return _call(f"读取远端 {name!r}", git.remote_url, name)


def _is_our_repo(url: str | None) -> bool:
    """远端 URL 是否指向本仓库的 GitHub 地址；SSH 与 HTTPS 写法都归一后比 host + owner/name"""
    if not url:
        return False
    parts = re.split(r"[/:]", url.strip().rstrip("/").removesuffix(".git"))
    if len(parts) < 3:
        return False
    host = parts[-3].rsplit("@", 1)[-1].lower()
    repository = f"{parts[-2]}/{parts[-1]}".lower()
    return host == "github.com" and repository == GITHUB_REPO.lower()
