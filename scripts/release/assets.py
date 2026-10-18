#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
产物发现
"""

from pathlib import Path

from build.packager import asset_name
from common import support
from common.entity.asset import Asset
from common.entity.target import SupportedTarget
from common.entity.version import Version
from common.enums.action import Action
from common.enums.kind import Kind
from common.enums.mode import Mode


class AssetDiscoveryError(Exception):
    """产物目录不完整或混入无法识别的文件"""


# 清单 / 签名 / 哈希 / 说明文件不是「产物」，不参与未知文件检查
_IGNORED_SUFFIXES = frozenset({".sig", ".sha256", ".json", ".md"})


def _expected(entry: SupportedTarget, version: Version, kinds: frozenset[Kind], directory: Path) -> list[tuple[Kind, Path]]:
    """按 Kind 声明顺序给出该 target 在指定模式下应产出的文件"""
    result = []
    for kind in Kind:
        if kind in entry.kinds and kind in kinds:
            result.append((kind, directory / asset_name(version, entry.target, kind)))
    return result


def _require_no_unknown(directory: Path, expected_names: set[str]) -> None:
    """目录里只允许出现期望产物与 .sig/.sha256/.json/.md；其余一律报错"""
    unknown = []
    for path in directory.iterdir():
        if not path.is_file() or path.name.startswith("."):
            continue
        if path.suffix.lower() in _IGNORED_SUFFIXES:
            continue
        if path.name not in expected_names:
            unknown.append(path.name)
    if unknown:
        raise AssetDiscoveryError(
            f"{str(directory)!r} 中存在无法识别的文件: {sorted(unknown)!r}；"
            f"请确认产物命名，或清理与发布无关的文件"
        )


def _discover_ci(directory: Path, version: Version) -> list[Asset]:
    """CI 模式下，目录必须覆盖同时开放 BUILD 与 RELEASE 的目标及其 RELEASE_KINDS"""
    if not directory.is_dir():
        raise AssetDiscoveryError(f"产物目录不存在: {str(directory)!r}")

    entries = support.pipeline_entries()
    if not entries:
        raise AssetDiscoveryError("支持表中没有同时开放 BUILD 与 RELEASE 的目标")

    assets: list[Asset] = []
    missing: list[str] = []
    expected_names: set[str] = set()

    for entry in entries:
        expected = _expected(entry, version, support.RELEASE_KINDS, directory)
        if not expected:
            raise AssetDiscoveryError(f"RELEASE 目标 {entry.target.triple!r} 没有任何 RELEASE_KINDS 载荷")
        for kind, path in expected:
            expected_names.add(path.name)
            if path.is_file():
                assets.append(Asset(entry.target, kind, path))
            else:
                missing.append(path.name)

    # [TODO]：此处对于missing，最好也能展示三元组，这要求missing类型为 dict[Target, str]
    if missing:
        names = "\n".join(f"{repr(name)}" for name in missing)
        raise AssetDiscoveryError(f"在 {str(directory)!r} 中缺少 {len(missing)} 个期望产物: {names}")

    _require_no_unknown(directory, expected_names)
    return assets


def _discover_local(directory: Path, version: Version) -> list[Asset]:
    """LOCAL 模式下，从实际文件反推单一 target，再要求该 target 的 LOCAL_KINDS 齐全"""
    if not directory.is_dir():
        raise AssetDiscoveryError(f"产物目录不存在: {str(directory)!r}")

    entries = support.entries(Action.BUILD)
    if not entries:
        raise AssetDiscoveryError("支持表中没有开放 BUILD 档位的目标")

    candidates: dict[str, tuple[SupportedTarget, list[tuple[Kind, Path]]]] = {}
    for entry in entries:
        expected = _expected(entry, version, support.LOCAL_KINDS, directory)
        if not expected:
            continue
        if any(path.is_file() for _kind, path in expected):
            candidates[entry.target.triple] = (entry, expected)

    if not candidates:
        raise AssetDiscoveryError(f"在 {str(directory)!r} 中未找到任何可识别的符合版本号 {version} 的本地构建产物")
    if len(candidates) > 1:
        raise AssetDiscoveryError(f"在 {str(directory)!r} 中同时出现多个目标的产物: {sorted(candidates)!r}；"
                                  f"本地发布目录应只包含一个目标")

    entry, expected = next(iter(candidates.values()))
    missing = [path.name for _kind, path in expected if not path.is_file()]
    if missing:
        names = "\n".join(f"{repr(name)}" for name in missing)
        raise AssetDiscoveryError(f"在 {str(directory)!r} 中对于目标三元组 {entry.target.triple!r} "
                                  f"缺少 {len(missing)} 个期望产物: {names}")

    _require_no_unknown(directory, {path.name for _kind, path in expected})
    return [Asset(entry.target, kind, path) for kind, path in expected]


def discover(directory: Path, version: Version, mode: Mode) -> list[Asset]:
    match mode:
        case Mode.LOCAL:
            return _discover_local(directory, version)
        case Mode.CI:
            return _discover_ci(directory, version)
