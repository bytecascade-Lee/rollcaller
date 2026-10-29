#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
GitHub Release 发布

流程：
    确保 tag 存在/ 创建 tag -> 创建 / 更新 Release -> 上传
"""

from collections.abc import Sequence
from pathlib import Path

from cli import gh
from common.entity.version import Version
from common.logger import info, success


class GithubReleaseError(Exception):
    """GitHub 发布流程的语义错误"""
    pass


def ensure_tag(repo: str, tag: str, sha: str) -> None:
    """确保 tag 存在：已存在则原样复用，不存在则在 sha 处创建

    不复核既有 tag 的指向：tag 触发时它由用户推送，目标提交以检出结果为准；
    手动重发同一版本时，也允许在既有 tag 上覆盖 Release 附件。
    """
    try:
        existing = gh.ref_commit_sha(repo, tag)
    except gh.GhError as e:
        raise GithubReleaseError(f"查询 GitHub tag {tag!r} 失败: {e}") from e

    if existing is not None:
        info(f"GitHub tag {tag!r} 已存在（指向 {existing!r}），沿用")
        return

    try:
        gh.create_tag(repo, tag, sha)
    except gh.GhError as e:
        raise GithubReleaseError(f"创建 GitHub tag {tag!r} 失败: {e}") from e
    success(f"已创建 GitHub tag {tag!r} -> {sha!r}")


def publish(repo: str, version: Version, tag: str, notes_file: Path, files: Sequence[Path], *, draft: bool) -> None:
    """创建或更新 Release"""
    prerelease = version.is_prerelease()
    title = str(version)
    info(
        f"repo: {repo!r} | version: {str(version)!r} | tag: {tag!r} | title: {title!r} | draft: {draft!r} | prerelease: {prerelease!r}")

    try:
        exists = gh.release_exists(repo, tag)
    except gh.GhError as e:
        raise GithubReleaseError(f"查询 GitHub Release {tag!r} 失败: {e}") from e

    if exists:
        try:
            gh.edit_release(
                repo, tag,
                title=title,
                notes_file=notes_file,
                draft=draft,
                prerelease=prerelease,
            )
            gh.upload_release_assets(repo, tag, files)
        except gh.GhError as e:
            raise GithubReleaseError(f"更新 GitHub Release {tag!r} 失败: {e}") from e
        success(f"已更新 GitHub Release {tag!r}，附件以覆盖的形式上传")
        return

    try:
        gh.create_release(
            repo, tag,
            title=title,
            notes_file=notes_file,
            draft=draft,
            prerelease=prerelease,
            files=files,
        )
    except gh.GhError as e:
        raise GithubReleaseError(f"创建 GitHub Release {tag!r} 失败: {e}") from e
    success(f"已创建 GitHub Release {tag!r}")
