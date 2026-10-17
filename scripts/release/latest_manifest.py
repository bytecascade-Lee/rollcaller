#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
自动更新清单 latest-*.json 的构造与序列化

JSON 契约符合 Rust 侧的 `crate::common::entity::update::UpdateManifest`，采用 serde camelCase

目前为：
    version / releaseNotes / publishDate / severity /
    platforms.windows.{x86_64|arm64}.{nsis|portable|develop}.{url, sha256, signature, size}
"""

import json
import re
from collections.abc import Sequence
from datetime import datetime, timezone
from pathlib import Path

from common.entity.asset import Asset
from common.entity.latest_manifest import Manifest, ManifestError, Payload
from common.entity.version import Version
from common.enums.arch import Arch
from common.enums.companion import Companion
from common.enums.kind import Kind
from common.enums.os import OS
from common.enums.severity import Severity
from common.enums.source import Source
from release import urls
from util.time import format_to_iso_8601


def _read_text(path: Path, label: str) -> str:
    if not path.is_file():
        raise ManifestError(f"缺少{label}: {str(path)!r}")
    text = path.read_text(encoding="UTF-8").strip()
    if not text:
        raise ManifestError(f"{label}为空: {str(path)!r}")
    return text


def _read_sha256(asset: Asset) -> str:
    path = asset.path.with_name(asset.path.name + Companion.SHA256.suffix)
    text = _read_text(path, "sha256 文件")
    if not re.fullmatch(r"[0-9a-fA-F]{64}", text):
        raise ManifestError(f"sha256 文件内容不是 64 位十六进制: {str(path)!r}")
    return text.lower()


def _read_signature(asset: Asset) -> str:
    return _read_text(asset.path.with_name(asset.path.name + Companion.SIGNATURE.suffix), "签名文件")


def payload(source: Source, version: Version, asset: Asset) -> Payload:
    """读取产物的伴生 .sig / .sha256 / size，构造单个载荷"""
    if not asset.path.is_file():
        raise ManifestError(f"产物文件不存在: {str(asset.path)!r}")
    return Payload(
        url=urls.asset_url(source, version, asset.path.name),
        sha256=_read_sha256(asset),
        signature=_read_signature(asset),
        size=asset.path.stat().st_size,
    )


def build(version: Version, notes: str, severity: Severity,
          assets: Sequence[Asset], source: Source, *,
          publish_date: datetime | None = None) -> Manifest:
    """把同一版本的产物按 os / arch / kind 聚合成清单实体"""
    if not assets:
        raise ManifestError("没有可写入清单的产物")

    platforms: dict[OS, dict[Arch, dict[Kind, Payload]]] = {}
    seen: set[tuple[str, str, str]] = set()

    for asset in assets:
        key = (str(asset.target.os), str(asset.target.arch), str(asset.kind))
        if key in seen:
            raise ManifestError(f"清单出现重复载荷: {key!r}")
        seen.add(key)

        arch_map = platforms.setdefault(asset.target.os, {}).setdefault(asset.target.arch, {})
        arch_map[asset.kind] = payload(source, version, asset)

    return Manifest(
        version=version,
        release_notes=notes,
        publish_date=publish_date or datetime.now(timezone.utc),
        severity=severity,
        platforms=platforms,
    )


def to_dict(manifest: Manifest) -> dict[str, object]:
    """序列化为 Rust `UpdateManifest` 期望的 camelCase 形状"""
    platforms: dict[str, object] = {}

    for os_name in OS:
        arches = manifest.platforms.get(os_name)
        if not arches:
            continue
        arch_dict: dict[str, object] = {}
        for arch in Arch:
            payloads = arches.get(arch)
            if not payloads:
                continue
            kind_dict: dict[str, object] = {}
            for kind in Kind:
                item = payloads.get(kind)
                if item is None:
                    continue
                kind_dict[str(kind)] = {
                    "url": item.url,
                    "sha256": item.sha256,
                    "signature": item.signature,
                    "size": item.size,
                }
            if kind_dict:
                arch_dict[arch.value] = kind_dict
        if arch_dict:
            platforms[os_name.value] = arch_dict

    if not platforms:
        raise ManifestError("清单序列化后 platforms 为空")

    return {
        "version": str(manifest.version),
        "releaseNotes": manifest.release_notes,
        "publishDate": format_to_iso_8601(manifest.publish_date),
        "severity": manifest.severity.value,
        "platforms": platforms,
    }


def write(manifest: Manifest, path: Path) -> Path:
    """写入 latest-*.json，2 空格缩进、UTF-8"""
    path.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(to_dict(manifest), ensure_ascii=False, indent=2)
    path.write_text(text, encoding="UTF-8")
    return path
