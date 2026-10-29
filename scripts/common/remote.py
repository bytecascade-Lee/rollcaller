#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
git 远端 URL 归一与角色判定

`git clone` 只是把 URL 原样写进 `.git/config`，所以远端 URL 是「你从哪儿克隆」这个事实，
不是权限：直接克隆规范仓库得到 `bytecascade-Lee/rollcaller`，克隆自己的 fork 得到 `<你>/rollcaller`。

不解析 host/port：URL 按 `[/:]` 切成段后取末两段就是 owner/repo，
端口（`github.com:22/o/r`）与 scp 形式（`git@github.com:o/r`）都被同一条规则吃掉；
"是不是本仓库"用常量仓库名做子串判断，`www.github.com`、`ssh://`、带端口的写法都无需特判。
"""

import re

from common.constant.repo import CNB_REPO, GITHUB_REPO
from common.enums.remote import RemoteRole

# 仓库名去 owner，用于识别 GitHub 上的同名仓库（fork）
REPO_NAME = GITHUB_REPO.rpartition("/")[2]

_SEGMENT_SEPARATORS = re.compile(r"[/:]")


def normalize(url: str) -> str:
    """去空白、末尾斜杠与 `.git` 后缀；不改大小写（GitHub 侧大小写不敏感，比较处各自 casefold）"""
    text = url.strip().rstrip("/")
    if text.casefold().endswith(".git"):
        text = text[: -len(".git")]
    return text


def owner_repo(url: str) -> str | None:
    """从 URL 里取 `owner/repo`（纯词法切分，只对 GitHub 地址有意义）；段数不足两个返回 None"""
    parts = [part for part in _SEGMENT_SEPARATORS.split(normalize(url)) if part]
    if len(parts) < 2:
        return None
    return f"{parts[-2]}/{parts[-1]}"


def is_github(url: str) -> bool:
    """URL 是否落在 github.com 上（`www.github.com`、带端口、ssh/scp 形式都算）"""
    return "github.com" in normalize(url).casefold()


def role_of(url: str) -> RemoteRole:
    """远端 URL 相对本仓库的角色；判定顺序不能换：先认规范仓库与镜像，剩下的同名 GitHub 仓库才是 fork"""
    text = normalize(url).casefold()
    if not text:
        return RemoteRole.UNRELATED
    if GITHUB_REPO.casefold() in text:
        return RemoteRole.CANONICAL
    if CNB_REPO.casefold() in text:
        return RemoteRole.MIRROR
    if is_github(text) and text.rpartition("/")[2] == REPO_NAME.casefold():
        return RemoteRole.FORK
    return RemoteRole.UNRELATED
