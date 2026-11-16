#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
服务目录的视图模型
"""

import json
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any

from common.constant.updater import DEVELOP_BASE, DEVELOP_PREFIX, VERSIONS_INDEX_FILENAME
from common.entity.latest_manifest import Manifest
from common.enums.source import Source
from util.str_format import join_repr

# 本地联调固定读 develop 那份清单
MANIFEST_FILENAME = Manifest.name(Source.DEVELOP)

# serve 目录内必须存在的文件
REQUIRED_FILES = (MANIFEST_FILENAME, VERSIONS_INDEX_FILENAME)

# 伴生文件后缀：签名与 sha256 都内嵌在清单里，客户端不会单独下载它们
COMPANION_SUFFIXES = (".sha256", ".sig")


class ServeError(Exception):
    """本地服务目录不满足要求，或清单读不出来"""


@dataclass(frozen=True, slots=True)
class AssetRow:
    """一份可下载的文件"""

    name: str
    href: str
    size_text: str


@dataclass(frozen=True, slots=True)
class EndpointRow:
    """一条对外路由"""

    path: str
    note: str
    size_text: str


@dataclass(frozen=True, slots=True)
class CheckRow:
    """一条自检结论"""

    text: str
    ok: bool


@dataclass(frozen=True, slots=True)
class IndexView:
    """首页要显示的全部数据"""

    origin: str
    version: str
    directory: Path
    published: str
    severity: str
    throttle: str
    checks: tuple[CheckRow, ...]
    endpoints: tuple[EndpointRow, ...]
    compat: tuple[EndpointRow, ...]
    assets: tuple[AssetRow, ...]
    companions: tuple[AssetRow, ...]


def _size_text(size: int) -> str:
    """人类可读的字节数"""
    if size < 1024:
        return f"{size} B"
    if size < 1024 * 1024:
        return f"{size / 1024:.1f} KiB"
    if size < 1024 * 1024 * 1024:
        return f"{size / (1024 * 1024):.1f} MiB"
    return f"{size / (1024 * 1024 * 1024):.2f} GiB"


def _size_of(path: Path) -> str:
    """文件的展示体积；不存在时返回「缺失」而不是抛错"""
    return _size_text(path.stat().st_size) if path.is_file() else "缺失"


def _text(value: object, field: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ServeError(f"清单字段 {field!r} 不是非空字符串: {value!r}")
    return value.strip()


def _local_time(value: object) -> str:
    """清单的 canonical ISO 8601 → 本地时间文本"""
    text = _text(value, "publishDate")
    try:
        moment = datetime.fromisoformat(text)
    except ValueError as error:
        raise ServeError(f"清单 publishDate 不是 ISO 8601: {text!r}") from error
    return moment.astimezone().strftime("%Y-%m-%d %H:%M")


def _read_manifest(serve_dir: Path) -> dict[str, Any]:
    path = serve_dir / MANIFEST_FILENAME
    try:
        data = json.loads(path.read_text(encoding="UTF-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ServeError(f"无法读取清单 {str(path)!r}: {error}") from error
    if not isinstance(data, dict):
        raise ServeError(f"清单 {str(path)!r} 的顶层不是对象")
    return data


def _payloads(manifest: dict[str, Any]) -> list[tuple[str, dict[str, Any]]]:
    """把 platforms.<os>.<arch>.<kind> 摊平成 (kind, payload) 列表"""
    platforms = manifest.get("platforms")
    if not isinstance(platforms, dict):
        raise ServeError("清单缺少 platforms 对象")

    flattened: list[tuple[str, dict[str, Any]]] = []
    for arches in platforms.values():
        if not isinstance(arches, dict):
            continue
        for payloads in arches.values():
            if not isinstance(payloads, dict):
                continue
            for kind, payload in payloads.items():
                if isinstance(payload, dict):
                    flattened.append((str(kind), payload))
    return flattened


def validate_dir(serve_dir: Path) -> None:
    """校验 serve 目录包含联调必需文件；缺失报错"""
    if not serve_dir.is_dir():
        raise ServeError(f"serve 目录不存在: {str(serve_dir)!r}")
    missing = [name for name in REQUIRED_FILES if not (serve_dir / name).is_file()]
    if missing:
        raise ServeError(
            f"目录 {str(serve_dir)!r} 缺少必要文件: {join_repr(missing)}；请先完成本地发布写入清单与版本索引"
        )


def inspect(serve_dir: Path, throttle_text: str) -> IndexView:
    """读清单 + 扫目录，产出首页视图

    每次请求都重新采集，页面上看到的就是磁盘现状；不计算 sha256 与签名——那是客户端的判定。
    """
    manifest = _read_manifest(serve_dir)
    version = _text(manifest.get("version"), "version")
    download = f"{DEVELOP_PREFIX}/releases/download/v{version}"
    latest = f"{DEVELOP_PREFIX}/releases/latest/download"

    missing: list[str] = []
    mismatched: list[str] = []
    origins: set[str] = set()
    assets: list[AssetRow] = []
    payloads = _payloads(manifest)

    for _, payload in payloads:
        url = _text(payload.get("url"), "platforms.*.url")
        name = url.rsplit("/", 1)[-1]
        origins.add(url.partition("/releases/")[0])

        target = serve_dir / name
        if not target.is_file():
            missing.append(name)
            continue
        actual = target.stat().st_size
        declared = payload.get("size")
        if isinstance(declared, int) and actual != declared:
            mismatched.append(f"{name}（清单 {_size_text(declared)} / 磁盘 {_size_text(actual)}）")
        assets.append(AssetRow(name=name, href=f"{download}/{name}", size_text=_size_text(actual)))

    checks: list[CheckRow] = []
    if not payloads:
        checks.append(CheckRow("清单里没有任何载荷", False))
    if missing:
        checks.append(CheckRow(f"清单引用的 {len(missing)} 个产物不在目录里：{join_repr(missing)}", False))
    if mismatched:
        checks.append(CheckRow(f"size 与清单不一致，客户端会拒装：{join_repr(mismatched)}", False))
    if payloads and not missing and not mismatched:
        checks.append(CheckRow(f"{len(assets)}/{len(payloads)} 个载荷均存在，size 与清单一致", True))

    stale = sorted(origin for origin in origins if origin != DEVELOP_BASE)
    if stale:
        checks.append(
            CheckRow(f"清单 URL 指向 {join_repr(stale)}，与当前监听 {DEVELOP_BASE} 不一致，请重跑 just release", False)
        )

    companions = tuple(
        AssetRow(name=path.name, href=f"{download}/{path.name}", size_text=_size_text(path.stat().st_size))
        for path in sorted(serve_dir.iterdir())
        if path.is_file() and path.name.endswith(COMPANION_SUFFIXES)
    )

    endpoints = (
        EndpointRow(
            path=f"{download}/{MANIFEST_FILENAME}",
            note="清单",
            size_text=_size_of(serve_dir / MANIFEST_FILENAME),
        ),
        EndpointRow(
            path=f"{latest}/{VERSIONS_INDEX_FILENAME}",
            note="索引",
            size_text=_size_of(serve_dir / VERSIONS_INDEX_FILENAME),
        ),
    )
    compat = (
        EndpointRow(
            path=f"{latest}/{MANIFEST_FILENAME}",
            note="",
            size_text=_size_of(serve_dir / MANIFEST_FILENAME),
        ),
    )

    return IndexView(
        origin=DEVELOP_BASE,
        version=version,
        directory=serve_dir,
        published=_local_time(manifest.get("publishDate")),
        severity=_text(manifest.get("severity"), "severity"),
        throttle=throttle_text,
        checks=tuple(checks),
        endpoints=endpoints,
        compat=compat,
        assets=tuple(assets),
        companions=companions,
    )
