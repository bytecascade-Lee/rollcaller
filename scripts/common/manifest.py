#!/usr/bin/env python3
"""
自动更新清单（latest.json）的构造与产物信息计算。

供 publish.py（真实发布：latest-github/cnb.json）与 release_local.py
（本地联调：latest-dev.json）共用，保证两端清单结构完全一致。

# 双写兼容结构

同一份清单同时携带新旧两套字段（对齐仓库根 `latest-compatible.json` 模板）：
- 旧组（早期发布格式）：`version / notes / pub_date / severity /
  platforms.{windows-x86_64|windows-aarch64}.{url, signature}`
  —— signature 是 minisign `.sig` 文本原文（历史语义，勿改）；
- 新组（Rust 解析器 `UpdateManifest`，serde camelCase）：
  `version / releaseNotes / publishDate / severity /
  platforms.windows.{x86_64|arm64}.{nsis|portable}.{url, sha256, signature, size}`
  —— signature 是 **base64(minisign `.sig` 全文)**，sha256 为十六进制小写，size 为字节数。

> 注意新老两组 signature 的编码语义不同，双写时务必分开处理（详见各函数文档）。
"""

import base64
import datetime
import hashlib
from pathlib import Path
from typing import Dict, Optional

# 版本索引/清单里 severity 的合法档位（与后端 Severity 枚举一致）
SEVERITY_LEVELS = ("normal", "important", "critical")

# 架构标识（manifest 内层用）→ 旧组 platform 键
LEGACY_PLATFORM_KEY = {"x86_64": "windows-x86_64", "arm64": "windows-aarch64"}

# manifest 内层架构键（新旧组一致）
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


def b64encode(text: str) -> str:
    """UTF-8 文本 → STANDARD base64（Rust 侧先解 base64 再解析内层文本）。"""
    return base64.b64encode(text.encode("utf-8")).decode("ascii")


def read_sig_text(sig_path: Optional[Path]) -> str:
    """读取 `.sig` 签名文件全文并去除首尾空白；文件缺失/为空返回 ""。

    `.sig` 是 minisign 文本：`untrusted comment:` / `trusted comment:` 头 + base64 签名体。
    旧组直接用该文本原文；新组用 [`b64encode`] 编码后再入清单。
    """
    if not sig_path or not sig_path.is_file():
        return ""
    return sig_path.read_text(encoding="utf-8").strip()


def build_artifact(url: str, path: Path, sig_raw: str = "") -> Dict:
    """构造单产物（新组 nsis/portable 的载荷对象）。

    Args:
        url: 产物下载直链（清单下发后客户端直接 GET）
        path: 本地产物文件路径（用于实算 sha256 与 size）
        sig_raw: `.sig` 全文（去除首尾空白）；为空则 signature=""（下载只验 sha256）

    Returns:
        {"url", "sha256", "signature", "size"}：signature 为 base64(minisign 文本)
    """
    return {
        "url": url,
        "sha256": sha256_hex(path),
        "signature": b64encode(sig_raw) if sig_raw else "",
        "size": path.stat().st_size,
    }


def build_latest_json(
    version: str,
    notes: str,
    severity: str,
    payloads: Dict[str, Dict],
    legacy_sigs: Optional[Dict[str, str]] = None,
    pub_date: Optional[str] = None,
) -> Dict:
    """构造双写兼容的自动更新清单。

    Args:
        version: 版本号（不含 v；本地联调用核心版本号，不带 +build）
        notes: 发布说明（旧组 notes 与新组 releaseNotes 共用）
        severity: normal | important | critical
        payloads: {arch: {"nsis": artifact, "portable": artifact}}
            artifact 由 [`build_artifact`] 产出；某形态缺省可给 None 或省略
        legacy_sigs: {arch: `.sig` 原文}，仅用于旧组 platform 的 signature
            （旧组 signature 语义=原文；缺省该 arch 时旧组 signature=""）
        pub_date: RFC3339 时间串（UTC，形如 2026-09-06T08:00:00Z）；
            缺省取当前 UTC 时间

    Returns:
        可直接 json.dumps 的清单 dict
    """
    if severity not in SEVERITY_LEVELS:
        fail(f"severity={severity!r} 非法（应为 {'/'.join(SEVERITY_LEVELS)}）")
    legacy_sigs = legacy_sigs or {}
    pub_date = pub_date or datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    # 新组：platforms.windows.{x86_64|arm64}.{nsis|portable}.{...}
    new_platforms = {}
    for arch in ARCHES:
        by_kind = payloads.get(arch) or {}
        entry = {}
        for kind in ("nsis", "portable"):
            artifact = by_kind.get(kind)
            if artifact:
                entry[kind] = artifact
        if entry:
            new_platforms.setdefault("windows", {})[arch] = entry

    # 旧组：platforms.{windows-x86_64|windows-aarch64}.{url, signature(原文)}
    legacy_platforms = {}
    for arch in ARCHES:
        nsis = (payloads.get(arch) or {}).get("nsis")
        if nsis:
            key = LEGACY_PLATFORM_KEY[arch]
            legacy_platforms[key] = {
                "url": nsis["url"],
                "signature": legacy_sigs.get(arch, ""),
            }

    return {
        "version": version,
        "notes": notes,
        "pub_date": pub_date,
        "releaseNotes": notes,
        "publishDate": pub_date,
        "severity": severity,
        "platforms": {
            **legacy_platforms,
            **new_platforms,
        },
    }
