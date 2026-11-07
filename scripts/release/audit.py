#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
发布物审计，逐源 × 逐版本

检查项：

    1. 线上 versions.json 能解析、且含该版本
    2. 清单字节的 sha256 == 索引条目 manifests.<source>.sha256
    3. 清单签名结构合法，且 key id 与客户端内置公钥一致
    4. 清单的 version / severity 与索引条目一致
    5. 清单里每个 url HEAD 必须 200；CNB 源的 url 还必须含 `/-/releases/`
"""

import base64
import binascii
import hashlib
import json
import re
import time
from collections.abc import Iterator
from dataclasses import dataclass

from common.constant.paths import BACKEND_DIR
from common.entity.version import Version
from common.entity.version_index import ManifestDigest, VersionIndex
from common.enums.source import Source
from common.logger import warning
from release import fetch, urls, versions_index

DEFAULT_TIMEOUT = fetch.DEFAULT_TIMEOUT
# 索引取不到目标版本时的重试间隔
RETRY_DELAY = 5
# CNB 的产物直链必须带 `/-/`，GitHub 反而不能有
CNB_PATH_MARKER = "/-/releases/"
# 客户端内置验签公钥所在处
PUBKEY_RS_PATH = BACKEND_DIR / "src" / "updater" / "constant.rs"
_PUBKEY_RE = re.compile(r'ROLLCALLER_UPDATE_PUBKEY:\s*&str\s*=\s*"([^"]+)"')
# minisign 签名载荷：2 字节算法 + 8 字节 key id + 64 字节签名
_MINISIGN_SIG_LEN = 74


class AuditError(Exception):
    """审计本身无法进行"""


@dataclass(frozen=True, slots=True)
class Failure:
    """一条失败项"""

    source: Source
    version: str
    item: str
    reason: str

    def render(self) -> str:
        return f"[{self.source.value}] {self.version} {self.item}: {self.reason}"


def expected_key_id() -> str:
    """客户端内置公钥的 key id"""
    if not PUBKEY_RS_PATH.is_file():
        raise AuditError(f"找不到内置公钥所在文件: {str(PUBKEY_RS_PATH)!r}")
    match = _PUBKEY_RE.search(PUBKEY_RS_PATH.read_text(encoding="UTF-8"))
    if match is None:
        raise AuditError(f"{str(PUBKEY_RS_PATH)!r} 里没有 ROLLCALLER_UPDATE_PUBKEY")

    try:
        # 常量内容是 base64(minisign 公钥文本)，公钥文本第二行才是 42 字节载荷
        pubkey_text = base64.b64decode(match.group(1)).decode("UTF-8")
        payload = base64.b64decode([line for line in pubkey_text.splitlines() if line.strip()][1])
    except (binascii.Error, UnicodeDecodeError, IndexError) as e:
        raise AuditError(f"内置公钥不是 base64(minisign 公钥文本): {e}") from e
    if len(payload) < 10:
        raise AuditError(f"内置公钥载荷只有 {len(payload)} 字节")
    return payload[2:10].hex()


def check_signature(signature: str, key_id: str) -> str | None:
    """签名结构 + key id 校验；通过返回 None，否则返回失败原因"""
    try:
        text = base64.b64decode(signature).decode("UTF-8")
    except (binascii.Error, UnicodeDecodeError) as e:
        return f"签名不是 base64(minisign 文本): {e}"

    lines = [line for line in text.splitlines() if line.strip()]
    if len(lines) < 2:
        return f"签名文本不足两行（实际 {len(lines)} 行）"
    try:
        payload = base64.b64decode(lines[1])
    except binascii.Error as e:
        return f"签名载荷不是合法 base64: {e}"
    if len(payload) != _MINISIGN_SIG_LEN:
        return f"签名载荷 {len(payload)} 字节，应为 {_MINISIGN_SIG_LEN}"

    actual = payload[2:10].hex()
    if actual != key_id:
        return f"签名 key id {actual} 与内置公钥 {key_id} 不一致（签错密钥？）"
    return None


def artifact_urls(manifest: dict) -> Iterator[tuple[str, str]]:
    """遍历清单里的全部产物：(路径标签, url)"""
    platforms = manifest.get("platforms")
    if not isinstance(platforms, dict):
        return
    for os_name, arches in platforms.items():
        if not isinstance(arches, dict):
            continue
        for arch, kinds in arches.items():
            if not isinstance(kinds, dict):
                continue
            for kind, payload in kinds.items():
                url = payload.get("url") if isinstance(payload, dict) else None
                if isinstance(url, str):
                    yield f"{os_name}/{arch}/{kind}", url


def check_version(source: Source, entry: VersionIndex, digest: ManifestDigest | None, key_id: str,
                  timeout: int) -> list[Failure]:
    """校验一个（源, 版本），返回失败项"""
    label = str(entry.version)
    failures: list[Failure] = []

    def fail(item: str, reason: str) -> None:
        failures.append(Failure(source, label, item, reason))

    if digest is None:
        warning(f"[{source.value}] {label} 索引条目没有 {source.value} 摘要，跳过摘要校验（legacy 条目）")

    url = urls.manifest_url(source, entry.version)
    status, body = fetch.request(url, timeout=timeout)
    if status != 200:
        fail("清单", f"HTTP {status}（{url}）")
        return failures

    if digest is not None:
        actual = hashlib.sha256(body).hexdigest()
        if actual != digest.sha256:
            fail("摘要", f"线上 {actual} != 索引 {digest.sha256}")
        if digest.signature:
            reason = check_signature(digest.signature, key_id)
            if reason is not None:
                fail("签名", reason)

    try:
        manifest = json.loads(body)
    except json.JSONDecodeError as e:
        fail("清单", f"不是合法 JSON: {e}")
        return failures

    if str(manifest.get("version")) != label:
        fail("版本", f"清单写的是 {manifest.get('version')!r}")
    if manifest.get("severity") != entry.severity.value:
        fail("severity", f"清单写的是 {manifest.get('severity')!r}，索引是 {entry.severity.value!r}")

    for item, target in artifact_urls(manifest):
        if source is Source.CNB and CNB_PATH_MARKER not in target:
            fail(item, f"CNB 直链缺少 {CNB_PATH_MARKER}：{target}")
            continue
        if source is Source.GITHUB and CNB_PATH_MARKER in target:
            fail(item, f"GitHub 直链不该带 {CNB_PATH_MARKER}：{target}")
            continue
        status, _ = fetch.request(target, method="HEAD", timeout=timeout)
        if status != 200:
            fail(item, f"HTTP {status}：{target}")

    return failures


def check_source(source: Source, want: Version | None, key_id: str, timeout: int, retry: int,
                 local_entries: list[VersionIndex] | None = None) -> list[Failure]:
    """校验一个源：先取索引（给了 `local_entries` 就用仓库索引），再逐版本校验"""
    if local_entries is not None:
        entries = local_entries
        if want is not None and versions_index.find(entries, want) is None:
            return [Failure(source, str(want), "versions.json", f"索引里没有 {want}")]
    else:
        entries, reason = remote_entries(source, want, retry, timeout)
        if entries is None:
            return [Failure(source, "-", "versions.json", reason or "取不到线上索引")]

    selected = [entry for entry in entries if want is None or entry.version == want]
    failures: list[Failure] = []
    for entry in selected:
        failures.extend(check_version(source, entry, entry.manifests.get(source), key_id, timeout))
    return failures


def remote_entries(source: Source, want: Version | None, retry: int,
                   timeout: int) -> tuple[list[VersionIndex] | None, str | None]:
    """拉线上版本索引"""
    url = urls.versions_index_url(source)
    reason = "取不到线上索引"
    for attempt in range(1, max(retry, 1) + 1):
        status, body = fetch.request(url, timeout=timeout)
        if status != 200:
            reason = f"HTTP {status}（{url}）"
        else:
            try:
                entries = versions_index.parse(body.decode("UTF-8"), label=f"[{source.value}] 版本索引")
            except (versions_index.VersionIndexError, UnicodeDecodeError) as e:
                reason = str(e)
            else:
                if want is None or versions_index.find(entries, want) is not None:
                    return entries, None
                reason = f"线上索引里没有 {want}（第 {attempt}/{retry} 次）"
        if attempt < retry:
            warning(f"[{source.value}] {reason}，{RETRY_DELAY} 秒后重试")
            time.sleep(RETRY_DELAY)
    return None, reason
