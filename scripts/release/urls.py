#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
自动更新 URL 构造

与 `backend/src/updater/urls.rs` 的 URL 模板一致：
    GitHub   https://github.com/<GITHUB_REPO>
    CNB      https://cnb.cool/<CNB_REPO>
    DEVELOP  http://localhost:14652/rollcaller（契约常量见 `common.constant.updater`）
"""

from common.constant.repo import CNB_REPO, GITHUB_REPO
from common.constant.updater import DEVELOP_BASE, VERSIONS_INDEX_FILENAME
from common.entity.latest_manifest import Manifest
from common.entity.version import Version
from common.enums.source import Source


def _base(source: Source) -> str:
    match source:
        case Source.GITHUB:
            return f"https://github.com/{GITHUB_REPO}"
        case Source.CNB:
            return f"https://cnb.cool/{CNB_REPO}/-"
        case Source.DEVELOP:
            return DEVELOP_BASE
        case _ as unreachable:
            raise AssertionError(unreachable)


def asset_url(source: Source, version: Version, file_name: str) -> str:
    """某个版本的产物直链：<base>/releases/download/v<version>/<name>"""
    return f"{_base(source)}/releases/download/{version.to_tag()}/{file_name}"


def versions_index_url(source: Source) -> str:
    """版本索引直链：<base>/releases/latest/download/versions.json"""
    return f"{_base(source)}/releases/latest/download/{VERSIONS_INDEX_FILENAME}"


def manifest_url(source: Source, version: Version) -> str:
    """某个版本的清单直链：<base>/releases/download/v<version>/latest-<source>.json"""
    return asset_url(source, version, Manifest.name(source))
