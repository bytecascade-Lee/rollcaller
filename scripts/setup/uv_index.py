#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
uv 默认源守卫
"""

import os
import tomllib
from pathlib import Path

from cli import uv
from common.constant.paths import PROJECT_BASE_DIR, PYPROJECT_PATH
from common.enums.os import OS
from common.logger import info, warning
from util.fs import relative_path
from util.str_format import join_repr
from .errors import EnvError

# uv 的默认源只允许声明在 pyproject.toml
UV_DEFAULT_INDEX_ENV = "UV_DEFAULT_INDEX"
UV_CONFIG_FILE_ENV = "UV_CONFIG_FILE"
_URL_PREFIXES = ("http://", "https://")


def problems() -> list[str]:
    """uv 的默认源必须与 pyproject.toml 声明的一致，否则 uv.lock 会被按别的源重写"""
    project_index = _project_index()
    if project_index is None:
        return []

    conflicts = _conflicting_indexes(project_index)
    if not conflicts:
        info(f"uv 源与本仓库一致: {project_index}")
        return []
    return [
        f"{item}={url}，与本仓库声明的 {project_index} 不一致；源不一致会让 uv 在 sync 时重写 uv.lock"
        for item, url in conflicts
    ]


def apply() -> None:
    """源不一致直接失败"""
    conflicts = problems()
    if conflicts:
        raise EnvError("uv 源配置与本仓库冲突: " + join_repr(conflicts))
    _report_lock_state()


def _project_index() -> str | None:
    """从 pyproject.toml 读 [[tool.uv.index]] 中 default = true 的地址"""
    try:
        data = tomllib.loads(PYPROJECT_PATH.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise EnvError(f"读取 {relative_path(PYPROJECT_PATH)} 失败: {error}") from error

    indexes = data.get("tool", {}).get("uv", {}).get("index", [])
    if isinstance(indexes, dict):
        indexes = [indexes]
    if not indexes:
        return None
    default = [entry for entry in indexes if isinstance(entry, dict) and entry.get("default")]
    chosen = default[0] if default else indexes[0]
    url = chosen.get("url") if isinstance(chosen, dict) else None
    return url if isinstance(url, str) else None


def _conflicting_indexes(project_index: str) -> list[tuple[str, str]]:
    """收集本机上指向别的源的 uv 配置，返回 (来源描述, 地址)"""
    expected = _normalize(project_index)
    conflicts: list[tuple[str, str]] = []

    env_value = os.environ.get(UV_DEFAULT_INDEX_ENV)
    if env_value and _normalize(env_value) != expected:
        conflicts.append((f"环境变量 {UV_DEFAULT_INDEX_ENV}", env_value))

    for path in _config_candidates():
        if not path.is_file():
            continue
        label = relative_path(path)
        try:
            data = tomllib.loads(path.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as error:
            raise EnvError(f"读取 uv 配置 {label} 失败: {error}") from error
        for url in _declared_defaults(data):
            if _normalize(url) != expected:
                conflicts.append((f"{label} 的默认源", url))
    return conflicts


def _declared_defaults(data: dict) -> list[str]:
    """从一份 uv 配置里取出「默认源」的地址"""
    values: list[str] = []
    for key in ("default-index", "index-url"):
        value = data.get(key)
        if isinstance(value, str) and value.startswith(_URL_PREFIXES):
            values.append(value)

    entries = data.get("index") or []
    if isinstance(entries, dict):
        entries = [entries]
    for entry in entries:
        if isinstance(entry, dict) and entry.get("default"):
            url = entry.get("url")
            if isinstance(url, str) and url.startswith(_URL_PREFIXES):
                values.append(url)
    return values


def _config_candidates() -> list[Path]:
    """uv 会读取的配置文件：显式指定的优先，其次是平台用户配置，最后是仓库根的同名文件"""
    explicit = os.environ.get(UV_CONFIG_FILE_ENV)
    if explicit:
        return [Path(explicit)]

    candidates: list[Path] = []
    if OS.current() == OS.WINDOWS:
        appdata = os.environ.get("APPDATA")
        if appdata:
            candidates.append(Path(appdata) / "uv" / "uv.toml")
    else:
        xdg = os.environ.get("XDG_CONFIG_HOME")
        base = Path(xdg) if xdg else Path.home() / ".config"
        candidates.append(base / "uv" / "uv.toml")
    candidates.append(PROJECT_BASE_DIR / "uv.toml")
    return candidates


def _normalize(url: str) -> str:
    """比较前归一：去首尾空白与结尾斜杠，大小写不敏感"""
    return url.strip().rstrip("/").lower()


def _report_lock_state() -> None:
    """uv.lock 与 pyproject.toml 是否一致"""
    if uv.lock_check():
        info("uv.lock 与 pyproject.toml 一致")
        return
    warning("uv.lock 已过期，依赖集合可能与仓库声明不同；请运行 `uv lock` 并提交结果")
