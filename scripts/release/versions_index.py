#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
versions.json 版本索引文件的读写与合并

索引是裸数组：
```
[
  {
    "version": "0.8.5",
    "severity": "normal",
    "manifests": {
      "github": {
        "sha256": "...",
        "signature": "..."
      },
      "cnb": {
        "sha256": "...",
        "signature": "..."
      }
    }
  },
  {"version": "0.8.2", "severity": "normal"}
]
```
"""

import json
from collections.abc import Mapping, Sequence
from pathlib import Path

from common.entity.version import Version
from common.entity.version_index import ManifestDigest, VersionIndex
from common.enums.severity import Severity
from common.enums.source import Source


class VersionIndexError(Exception):
    """versions.json 解析 / 写入错误"""


# 版本索引文件名
FILENAME = "versions.json"


def _unique(entries: Sequence[VersionIndex]) -> list[VersionIndex]:
    """按版本字符串去重，后者覆盖前者，保留首次出现的位置"""
    return list({str(entry.version): entry for entry in entries}.values())


def _ordered(entries: Sequence[VersionIndex]) -> list[VersionIndex]:
    """去重并按版本倒序"""
    return sorted(_unique(entries), key=lambda entry: entry.version, reverse=True)


def parse(text: str, label: str = "版本索引") -> list[VersionIndex]:
    """解析索引文本"""
    try:
        raw = json.loads(text)
    except json.JSONDecodeError as e:
        raise VersionIndexError(f"{label} 解析失败: {e}") from e
    if not isinstance(raw, list):
        raise VersionIndexError(f"{label} 不是合法的裸数组版本索引")

    try:
        entries = [VersionIndex.from_dict(item) for item in raw]
    except ValueError as e:
        raise VersionIndexError(f"{label} 条目非法: {e}") from e
    return _unique(entries)


def read(path: Path) -> list[VersionIndex]:
    """读取数组索引，重复版本取最后出现的"""
    if not path.is_file():
        raise VersionIndexError(f"版本索引文件不存在: {str(path)!r}")
    return parse(path.read_text(encoding="UTF-8"), label=f"{str(path)!r}")


def write(path: Path, entries: Sequence[VersionIndex]) -> Path:
    """去重后按版本倒序、缩进 2、**LF 行尾**写入裸数组索引"""
    payload = [entry.to_dict() for entry in _ordered(entries)]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="UTF-8", newline="\n")
    return path


def merge(entries: Sequence[VersionIndex], version: Version, severity: Severity,
          manifests: Mapping[Source, ManifestDigest] | None = None) -> list[VersionIndex]:
    """合并本次版本，相同版本的 severity / manifests 被替换，其余条目原样保留"""
    existing = find(entries, version)
    merged_manifests = manifests if manifests is not None else (existing.manifests if existing else {})
    key = str(version)
    merged = [entry for entry in entries if str(entry.version) != key]
    merged.append(VersionIndex(version=version, severity=severity, manifests=merged_manifests))
    return _ordered(merged)


def assert_preserved(before: Sequence[VersionIndex], after: Sequence[VersionIndex], version: Version) -> None:
    """除本次版本外，每个条目的 manifests 必须逐项不变"""
    key = str(version)
    after_map = {str(entry.version): entry for entry in after}
    for entry in before:
        if str(entry.version) == key:
            continue
        current = after_map.get(str(entry.version))
        if current is None:
            raise VersionIndexError(f"版本索引写入后丢失条目 {entry.version}")
        if dict(current.manifests) != dict(entry.manifests):
            raise VersionIndexError(f"版本索引写入后 {entry.version} 的清单摘要被改动，疑似解析/序列化丢字段")


def find(entries: Sequence[VersionIndex], version: Version) -> VersionIndex | None:
    """查找指定版本，若不存在返回 None"""
    key = str(version)
    return next((entry for entry in entries if str(entry.version) == key), None)
