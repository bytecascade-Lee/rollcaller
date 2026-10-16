#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
版本号文件的临时改写与还原
"""

import re
from pathlib import Path

from cli import git, uv
from common.constant.paths import (
    CARGO_TOML_PATH,
    PACKAGE_JSON_PATH,
    PROJECT_BASE_DIR,
    PYPROJECT_PATH,
    TAURI_CONF_PATH,
    VERSION_TRACKED_FILES,
)
from common.entity.version import Version
from common.logger import info


class VersionFileError(Exception):
    """版本号文件改写 / 还原相关错误"""


# 各版本文件的版本号字段改写规则
_REWRITE_RULES = (
    (PYPROJECT_PATH, re.compile(r'^(version\s*=\s*")([^"]*)(")', re.MULTILINE)),
    (TAURI_CONF_PATH, re.compile(r'^(\s*version\s*:\s*")([^"]*)(")', re.MULTILINE)),
    (CARGO_TOML_PATH, re.compile(r'^(version\s*=\s*")([^"]*)(")', re.MULTILINE)),
    (PACKAGE_JSON_PATH, re.compile(r'^(\s*"version"\s*:\s*")([^"]*)(")', re.MULTILINE)),
)


def check_git_status() -> bool:
    """版本文件与锁文件相对 HEAD 是否无未提交改动"""
    clean, _detail = git.are_clean(VERSION_TRACKED_FILES, cwd=PROJECT_BASE_DIR)
    return clean


def sync(version: Version) -> None:
    """把版本号写入各版本文件，并刷新 uv 锁文件"""
    for path, pattern in _REWRITE_RULES:
        _rewrite(path, pattern, str(version))
    uv.lock()
    info("已同步版本号并刷新 uv 锁文件")


def restore() -> None:
    """用 git 把版本文件与锁文件还原到 HEAD，丢弃 sync 的临时改动"""
    git.restore_files(VERSION_TRACKED_FILES, cwd=PROJECT_BASE_DIR)
    info("已还原版本文件与 uv 锁文件")


def _rewrite(path: Path, pattern: re.Pattern[str], version: str) -> None:
    """按行匹配把单个文件的版本号字段改写为 version"""
    if not path.is_file():
        raise VersionFileError(f"版本文件不存在: {str(path)!r}")
    content = path.read_text(encoding="utf-8")
    match = pattern.search(content)
    if match is None:
        raise VersionFileError(f"未在 {str(path)!r} 中找到版本号字段，文件格式可能已变化")
    old = match.group(2)
    label = path.relative_to(PROJECT_BASE_DIR).as_posix()
    if old == version:
        info(f"{label}: 版本号已是 {version}，无需修改")
        return
    new_content = pattern.sub(lambda m: m.group(1) + version + m.group(3), content, count=1)
    path.write_text(new_content, encoding="utf-8")
    info(f"{label}: {old} -> {version}")
