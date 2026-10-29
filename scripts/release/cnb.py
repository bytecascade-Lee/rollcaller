#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
CNB Release 发布

流程：
    1. 等待 sync-mirrors 把 tag 镜像到 CNB；超时后主动创建，再复核
    2. 按照 tag 查 Release，存在则 patch，不存在则 post
    3. 逐个上传附件
"""

import time
from collections.abc import Sequence
from pathlib import Path

from cli import cnb
from common.entity.version import Version
from common.logger import info, success, warning


class CnbReleaseError(Exception):
    """CNB 发布流程的语义错误"""


# 等待 sync-mirrors 把 tag 镜像到 CNB 的窗口（秒）
MIRROR_WAIT_TIMEOUT = 60
# 主动创建 tag 后的复核窗口（秒）
TAG_CONFIRM_TIMEOUT = 60
# tag 探测轮询间隔（秒）
TAG_POLL_INTERVAL = 15


def _wait_tag(repo: str, tag: str, timeout: int, interval: int) -> bool:
    """在 timeout 秒内以 interval 间隔轮询 CNB 上的 tag"""
    deadline = time.monotonic() + timeout
    count = 1
    while time.monotonic() < deadline:
        if cnb.tag_exists(repo, tag):
            info(f"在第 {count} 次轮询时，检测到 tag 已创建")
            return True
        info(f"在第 {count} 次轮询时，检测到 tag 还未创建，继续轮询...")
        count += 1
        time.sleep(interval)
    warning(f"经过 {count} 次轮询后，tag 仍未成功创建")
    return False


def ensure_tag(repo: str, tag: str, sha: str) -> None:
    """确保 CNB 上存在 tag：先等镜像，超时主动创建，最后复核。"""
    if _wait_tag(repo, tag, timeout=MIRROR_WAIT_TIMEOUT, interval=TAG_POLL_INTERVAL):
        info(f"CNB 已同步 tag {tag!r} sync-mirrors")
        return

    info(f"CNB 在 {MIRROR_WAIT_TIMEOUT}s 内未见 tag {tag!r}，改用 cnb api 手动创建，对应 target 为 {sha!r}）")
    try:
        cnb.create_tag(repo, tag, sha)
    except cnb.CnbError as e:
        # 典型情况是并发下 sync-mirrors 恰好把 tag 补上，交由下面的复核定论
        warning(f"手动创建 tag 未成功: {e}")

    if _wait_tag(repo, tag, timeout=TAG_CONFIRM_TIMEOUT, interval=TAG_POLL_INTERVAL):
        success(f"CNB tag {tag!r} 已就绪")
        return
    raise CnbReleaseError(f"CNB 上仍无 tag {tag!r}，对应 target 为 {sha!r}，请确认该提交已镜像到 CNB")


def publish(repo: str, version: Version, tag: str, notes_file: Path, files: Sequence[Path], *, draft: bool) -> None:
    """创建或更新 Release 并上传附件"""
    prerelease = version.is_prerelease()
    make_latest = not (draft or prerelease)
    name = str(version)
    info(f"repo: {repo!r} | version: {str(version)!r} | tag: {tag!r} | name: {name!r} | "
         f"draft: {draft!r} | prerelease: {prerelease!r} | latest: {make_latest}")

    try:
        release_id = cnb.release_id_by_tag(repo, tag)
        if release_id:
            cnb.update_release(
                repo, release_id,
                name=name,
                body_file=notes_file,
                make_latest=make_latest,
                draft=draft,
                prerelease=prerelease,
            )
            info(f"已更新 CNB Release {release_id}，对应 tag 为 {tag!r}")
        else:
            release_id = cnb.create_release(
                repo, tag,
                name=name,
                body_file=notes_file,
                make_latest=make_latest,
                draft=draft,
                prerelease=prerelease,
            )
            info(f"已创建 CNB Release {release_id}，对应 tag 为 {tag!r}")

        for path in files:
            cnb.upload_release_asset(repo, release_id, path)
            info(f"已上传文件 {str(path)!r} 到 release {release_id!r}")
    except cnb.CnbError as e:
        raise CnbReleaseError(f"发布 CNB Release {tag!r} 失败: {e}") from e

    success(f"CNB Release 同步完成: https://cnb.cool/{repo}/-/releases/tag/{tag}")
