#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
本地构建目录定位

本地发布的版本号是 `<core>+<branch>.<count>.<hash>`，目录名必须与当前
HEAD 推导出的 full 版本**完全一致**：只按 core 前缀择优会掩盖「构建后
又切分支 / 产生新提交」这个错误。
"""

from dataclasses import dataclass
from pathlib import Path

from common.constant.paths import LOCAL_RELEASE_DIR
from common.entity.version import Version


class LocalReleaseError(Exception):
    """本地构建目录不存在或与当前提交不一致"""


@dataclass(frozen=True, slots=True)
class LocalBuild:
    version: Version
    directory: Path


def find(full_version: Version) -> LocalBuild:
    """按 full 版本精确定位 `release/Local/<full>`；缺失时列出同 core 的目录"""
    if full_version.build is None:
        raise LocalReleaseError(f"本地发布需要带构建元数据的 full 版本号: {full_version!r}")

    directory = LOCAL_RELEASE_DIR / str(full_version)
    if directory.is_dir():
        return LocalBuild(full_version, directory)

    core = str(full_version).split("+", 1)[0]
    candidates = sorted(path.name for path in LOCAL_RELEASE_DIR.glob(f"{core}+*") if path.is_dir())
    hint = (f"现有同核心版本目录: {candidates!r}" if candidates else f"{str(LOCAL_RELEASE_DIR)!r} 下没有同核心版本的目录")
    raise LocalReleaseError(
        f"未找到本地构建目录 {str(directory)!r}。\n"
        f"如果构建后切换过分支或产生过新提交，full 版本会变化，请重新构建；{hint}"
    )
