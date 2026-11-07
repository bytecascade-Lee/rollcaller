#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
把线上清单的 sha256 回填进仓库版本索引（resources/update/versions.json）

# 为什么要有这一步

摘要由发布流水线算出来、写进**线上**的 versions.json；仓库里的那份是**下一次**发布要读的
（`release_ci` 用 `read(仓库) → merge → write(assets)`）。所以线上那份新算的摘要必须回到仓库，
否则下一个版本发布时它就不在索引里了。

`bump` 的闸门会检查"所有更早版本的两源摘要齐全"，缺了直接拒绝打 tag——本模块就是补这一步。

# 数据来源（顺序不可换）

- **sha256 永远以线上清单字节重算为准**，再与线上索引里的值比对；不一致直接报错
  （说明线上索引与线上清单已经打架，先把发布修好，别把错的抄回仓库）；
- **签名**取自线上索引条目（历史清单没签过，新版本才有）。

# 两条路径

- `collect()` 纯计算：算完返回新条目，不碰文件、不碰 git；
- `propose()` 只服务 `--pr`：提交到机器人分支并开 PR。**这是本模块唯一会改 git 状态的路径**，
  本地不带 `--pr` 时永远不会走到。
"""

import hashlib
from collections.abc import Sequence
from pathlib import Path

from cli import git
from common.constant.paths import PROJECT_BASE_DIR
from common.constant.repo import GITHUB_REPO
from common.entity.version import Version
from common.entity.version_index import ManifestDigest, VersionIndex
from common.enums.source import Source
from common.logger import info, success, warning
from release import fetch, github, urls, versions_index

DEFAULT_TIMEOUT = fetch.DEFAULT_TIMEOUT
# 正式发布使用的源（develop 是本地联调索引，不参与回填）
DIGEST_SOURCES = (Source.GITHUB, Source.CNB)
# 流水线的提交身份：走 env 注入而不是 `git config`，本地跑一次也不会留下别人的身份
BOT_IDENTITY = {
    "GIT_AUTHOR_NAME": "github-actions[bot]",
    "GIT_AUTHOR_EMAIL": "41898282+github-actions[bot]@users.noreply.github.com",
    "GIT_COMMITTER_NAME": "github-actions[bot]",
    "GIT_COMMITTER_EMAIL": "41898282+github-actions[bot]@users.noreply.github.com",
}


class BackfillError(Exception):
    """回填过程中发现线上与仓库不一致，或数据取不到"""


def snapshot(entries: Sequence[VersionIndex]) -> dict[str, dict]:
    """按版本号取序列化快照，用来判断"到底改没改"（与文件里的书写顺序无关）"""
    return {str(entry.version): entry.to_dict() for entry in entries}


def _fetch_index(url: str, label: str, timeout: int) -> list[VersionIndex] | None:
    """取一份索引并解析；任何失败都返回 None（调用方决定是告警还是回退）"""
    status, body = fetch.request(url, timeout=timeout)
    if status != 200:
        warning(f"{label} 取不到（HTTP {status}）：{url}")
        return None
    try:
        return versions_index.parse(body.decode("UTF-8"), label=label)
    except (versions_index.VersionIndexError, UnicodeDecodeError) as e:
        warning(f"{label} 不可用（{e}）：{url}")
        return None


def live_entries(source: Source, wanted: Sequence[Version], timeout: int) -> list[VersionIndex]:
    """线上索引的条目

    首选 `releases/latest/download/versions.json`（一次性含全部历史）。但它**只指向最新正式版**：
    预发布 `make_latest=false`，所以 rc 根本不在这份索引里；发布刚结束时它也还可能是旧的。
    因此当目标版本不在里面时，回落到该版本 Release 自带的 `versions.json`（内容同构，且一定含自己）。
    两条路都拿不到才告警——那时摘要仍可算，只是没有签名可抄。
    """
    entries = _fetch_index(urls.versions_index_url(source), f"[{source.value}] 线上版本索引", timeout) or []
    missing = [version for version in wanted if versions_index.find(entries, version) is None]
    if not missing:
        return entries

    for version in missing:
        fallback = _fetch_index(
            urls.asset_url(source, version, versions_index.FILENAME),
            f"[{source.value}] {version} 的版本索引",
            timeout,
        )
        if fallback is not None:
            info(f"[{source.value}] latest 索引不含 {version}（预发布或尚未传播），改用该 Release 自带的 versions.json")
            return fallback

    warning(f"[{source.value}] 线上索引不含 {'、'.join(str(v) for v in missing)}，本次回填不带签名")
    return entries


def digest_of(source: Source, version: Version, published: ManifestDigest | None,
              timeout: int) -> ManifestDigest:
    """按线上清单字节算摘要，并与线上索引里的值交叉核对"""
    url = urls.manifest_url(source, version)
    status, body = fetch.request(url, timeout=timeout)
    if status != 200:
        raise BackfillError(f"[{source.value}] {version} 取不到清单（HTTP {status}）：{url}")

    actual = hashlib.sha256(body).hexdigest()
    if published is None:
        # 线上索引里没有该源摘要（历史清单从未签过名）⇒ 只回填 sha256
        info(f"{version} {source.value}: {actual[:16]}… 仅摘要（线上索引无此摘要，无从取签名）")
        return ManifestDigest(sha256=actual)
    if published.sha256 != actual:
        raise BackfillError(
            f"[{source.value}] {version} 线上清单与线上索引的摘要不一致："
            f"清单 {actual} / 索引 {published.sha256}；先修发布，别把错的抄回仓库"
        )
    info(f"{version} {source.value}: {actual[:16]}… {'签名+摘要' if published.signature else '仅摘要'}")
    return ManifestDigest(sha256=actual, signature=published.signature)


def collect(entries: Sequence[VersionIndex], sources: Sequence[Source],
            wanted: Sequence[Version] | None, timeout: int) -> list[VersionIndex]:
    """重算各源摘要，返回回填后的完整条目列表

    纯计算：不写文件、不碰 git。`wanted` 之外的条目原样带回。
    """
    if wanted is not None:
        absent = [str(version) for version in wanted if versions_index.find(entries, version) is None]
        if absent:
            raise BackfillError(f"索引里没有这些版本：{'、'.join(absent)}")

    selected = [entry for entry in entries if wanted is None or entry.version in wanted]
    published = {source: live_entries(source, wanted or [], timeout) for source in sources}

    refreshed: dict[str, VersionIndex] = {}
    for entry in selected:
        manifests = dict(entry.manifests)
        for source in sources:
            live = versions_index.find(published[source], entry.version)
            manifests[source] = digest_of(source, entry.version, live.manifests.get(source) if live else None, timeout)
        refreshed[str(entry.version)] = VersionIndex(
            version=entry.version, severity=entry.severity, manifests=manifests)

    return [refreshed.get(str(entry.version), entry) for entry in entries]


def _pull_request_body(version: Version, entries: Sequence[VersionIndex]) -> str:
    """PR 正文：带上本次回填的摘要，省得 reviewer 再去线上抄一遍"""
    entry = versions_index.find(entries, version)
    rows = ["| 源 | sha256 | 签名 |", "|:---|:---|:---|"]
    for source in DIGEST_SOURCES:
        digest = entry.manifests.get(source) if entry is not None else None
        if digest is None:
            continue
        rows.append(f"| {source.value} | `{digest.sha256}` | {'有' if digest.signature else '无'} |")
    table = "\n".join(rows)

    return (
        f"由发布流水线自动生成。改动只有 `resources/update/versions.json` 里 {version} 条目的 "
        f"`manifests` 字段（sha256 + 签名文本），值取自线上清单字节与线上索引。\n\n"
        f"{table}\n\n"
        "**为什么必须合入**：仓库索引是下一次发布读取摘要的唯一来源；"
        "不合入则 `just bump` 的「更早版本摘要齐全」闸门会拒绝打 tag。\n\n"
        "**合入前可自行复核**：`just verify-release -i resources/update/versions.json`"
        "（两端逐版本核对摘要，并检查每个产物直链是否 200）。\n\n"
        "GITHUB_TOKEN 创建的 PR 不会触发 CI，所以这里不指望自动检查。"
    )


def propose(version: Version, index_path: Path, entries: Sequence[VersionIndex], *,
            base: str = "develop") -> None:
    """提交回填结果并开 PR

    **只有 `--pr` 会走到这里，且它是本模块唯一改 git 状态的路径。** 三条自我约束：

    - 只 `git add` 一个文件：工作区若还有别的改动立即失败，免得把半成品一起提交；
    - 只推带版本号的机器人分支（`--force` 覆盖同名分支，重跑幂等）；
    - 已有打开的 PR 时只更新分支，不重复开。
    """
    branch = f"chore/backfill-versions-{version}"
    try:
        relative = index_path.resolve().relative_to(PROJECT_BASE_DIR.resolve()).as_posix()
    except ValueError:
        relative = index_path.as_posix()

    others = [
        line for line in git.tracked_changes().splitlines()
        if line.strip() and not line.endswith(relative)
    ]
    if others:
        raise BackfillError(f"--pr 要求工作区只有 {relative!r} 有改动，实际还有：{'；'.join(others)}")

    git.checkout_branch(branch, force=True)
    git.add([relative])
    git.commit(f"chore(release): 回填 {version} 的清单摘要", env=BOT_IDENTITY)
    git.push_branch(branch, force=True)
    success(f"已推送分支 {branch}")

    github.ensure_pull_request(
        GITHUB_REPO,
        branch,
        base,
        f"chore(release): 回填 {version} 的清单摘要",
        _pull_request_body(version, entries),
    )
