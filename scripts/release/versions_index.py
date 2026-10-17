#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
versions.json 版本索引文件的读写与合并

索引是裸数组：

    [
        {"version": "0.8.2", "severity": "normal"},
        {"version": "0.8.1", "severity": "normal"}
    ]
"""

import json
from collections.abc import Sequence
from pathlib import Path

from common.entity.version import Version
from common.entity.version_index import VersionIndex
from common.enums.severity import Severity


class VersionIndexError(Exception):
    """versions.json 解析 / 写入错误"""


def _unique(entries: Sequence[VersionIndex]) -> list[VersionIndex]:
    """按版本字符串去重，后者覆盖前者，保留首次出现的位置"""
    return list({str(entry.version): entry for entry in entries}.values())


def _ordered(entries: Sequence[VersionIndex]) -> list[VersionIndex]:
    """去重并按版本倒序"""
    return sorted(_unique(entries), key=lambda entry: entry.version, reverse=True)


def read(path: Path) -> list[VersionIndex]:
    """读取数组索引，重复版本取最后出现的"""
    if not path.is_file():
        raise VersionIndexError(f"版本索引文件不存在: {str(path)!r}")
    try:
        raw = json.loads(path.read_text(encoding="UTF-8"))
    except json.JSONDecodeError as e:
        raise VersionIndexError(f"{str(path)!r} 解析失败: {e}") from e
    if not isinstance(raw, list):
        raise VersionIndexError(f"{str(path)!r} 不是合法的裸数组版本索引")

    try:
        entries = [VersionIndex.from_dict(item) for item in raw]
    except ValueError as e:
        raise VersionIndexError(f"{str(path)!r} 条目非法: {e}") from e
    return _unique(entries)


def write(path: Path, entries: Sequence[VersionIndex]) -> Path:
    """去重后按版本倒序、缩进 4 的方式写入裸数组索引"""
    payload = [entry.to_dict() for entry in _ordered(entries)]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=4), encoding="UTF-8")
    return path


def merge(entries: Sequence[VersionIndex], version: Version, severity: Severity) -> list[VersionIndex]:
    """合并本次版本，相同版本替换 severity，其余保留，返回按版本倒序的新列表"""
    key = str(version)
    merged = [entry for entry in entries if str(entry.version) != key]
    merged.append(VersionIndex(version=version, severity=severity))
    return _ordered(merged)


def find(entries: Sequence[VersionIndex], version: Version) -> VersionIndex | None:
    """查找指定版本，若不存在返回 None"""
    key = str(version)
    return next((entry for entry in entries if str(entry.version) == key), None)
