#!/usr/bin/env python3
"""
自动更新清单（latest-*.json）的构造与产物信息计算。

供 publish_ci.py（真实发布：latest-github/cnb.json）与 publish_local.py
（本地联调：latest-develop.json）共用，保证两端清单结构完全一致。

# v2 结构（Rust 解析器 `UpdateManifest`，serde camelCase；模板见仓库根 latest-v2.json）

    version / releaseNotes / publishDate / severity /
    platforms.windows.{x86_64|arm64}.{nsis|portable}.{url, sha256, signature, size}

- signature 直接取 **`.sig` 文件全文**：tauri signer ≥2.11 产出的 `.sig` 本身已是
  base64(minisign 签名文本，单行)——Rust 侧 base64 解码一次即得四行 minisign 文本再验签；
  空串表示该产物无签名（下载只验 sha256）。
  > 勿对已是 base64 的 `.sig` 再次编码（曾导致清单 signature 双重 base64、
  > Rust `Signature::decode` 报 InvalidEncoding）；若将来兼容旧版 CLI 的明文多行 `.sig`，
  > 请在读取处自行 base64，而不是在 [`build_artifact`] 里无条件编码。
- sha256 为十六进制小写，size 为字节数，均由本模块实算。

> v1 旧组字段（version/notes/pub_date/platforms.windows-x86_64.{url, signature 原文}）
> 已废弃：签名密钥轮换后旧公钥签名作废，历史格式无存在意义，不再双写兼容。
"""

import datetime
import hashlib
from pathlib import Path
from typing import Dict, Optional

# 版本索引/清单里 severity 的合法档位（与后端 Severity 枚举一致）
SEVERITY_LEVELS = ("normal", "important", "critical")

# manifest 内层架构键（与后端 Arch 枚举一致）
ARCHES = ("x86_64", "arm64")


def fail(message: str) -> None:
    """模块内错误统一抛 ValueError（由调用方决定终止方式）。"""
    raise ValueError(message)


def sha256_hex(path: Path) -> str:
    """文件 sha256 十六进制（小写），与 Rust `verify_sha256` 期望一致。"""
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_sig_text(sig_path: Optional[Path]) -> str:
    """读取 `.sig` 签名文件全文并去除首尾空白；文件缺失/为空返回 ""。

    `.sig` 是 tauri signer ≥2.11 的输出，内容已是 base64(minisign 签名文本)（单行），
    返回值直接作为清单 signature 字段（见 [`build_artifact`]），**无需再次编码**。
    """
    if not sig_path or not sig_path.is_file():
        return ""
    return sig_path.read_text(encoding="utf-8").strip()


def build_artifact(url: str, path: Path, sig_raw: str = "") -> Dict:
    """构造单产物载荷（platforms.windows.<arch>.<nsis|portable> 的值对象）。

    Args:
        url: 产物下载直链（清单下发后客户端直接 GET）
        path: 本地产物文件路径（用于实算 sha256 与 size）
        sig_raw: `.sig` 全文（去除首尾空白）；为空则 signature=""（下载只验 sha256）

    Returns:
        {"url", "sha256", "signature", "size"}：signature 直接取 `.sig` 全文
        （tauri signer ≥2.11 的 `.sig` 已含 base64(minisign 文本)，不再二次编码）
    """
    return {
        "url": url,
        "sha256": sha256_hex(path),
        "signature": sig_raw,
        "size": path.stat().st_size,
    }


def build_latest_json(
    version: str,
    notes: str,
    severity: str,
    payloads: Dict[str, Dict],
    pub_date: Optional[str] = None,
) -> Dict:
    """构造 v2 结构的自动更新清单（与 latest-v2.json 模板逐字段一致）。

    Args:
        version: 版本号（不含 v；发布/索引条目里的核心版本号，不带 +build）
        notes: 发布说明（releaseNotes）
        severity: normal | important | critical
        payloads: {arch: {"nsis"|"portable"|"develop": artifact}}
            artifact 由 [`build_artifact`] 产出；某形态缺省可给 None 或省略
            （develop = Develop 直更载荷，zip 内含 debug exe，文件名见 packager.DEVELOP_UPDATE_BIN_NAME）
        pub_date: RFC3339 时间串（UTC，形如 2026-09-06T08:00:00Z）；
            缺省取当前 UTC 时间

    Returns:
        可直接 json.dumps 的清单 dict
    """
    if severity not in SEVERITY_LEVELS:
        fail(f"severity={severity!r} 非法（应为 {'/'.join(SEVERITY_LEVELS)}）")
    pub_date = pub_date or datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    platforms = {}
    for arch in ARCHES:
        by_kind = payloads.get(arch) or {}
        entry = {}
        for kind in ("nsis", "portable", "develop"):
            artifact = by_kind.get(kind)
            if artifact:
                entry[kind] = artifact
        if entry:
            platforms.setdefault("windows", {})[arch] = entry

    return {
        "version": version,
        "releaseNotes": notes,
        "publishDate": pub_date,
        "severity": severity,
        "platforms": platforms,
    }
