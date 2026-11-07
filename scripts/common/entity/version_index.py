#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
版本索引
"""

from collections.abc import Mapping
from dataclasses import dataclass, field

from common.entity.version import Version
from common.enums.severity import Severity
from common.enums.source import Source
from util.str_format import join_repr

_MANIFEST_ORDER = (Source.GITHUB, Source.CNB, Source.DEVELOP)
_HEX_DIGITS = frozenset("0123456789abcdef")
_KNOWN_FIELDS = frozenset({"version", "severity", "manifests"})


def _parse_sha256(raw: object) -> str:
    """校验并规范化 sha256：64 位十六进制、统一小写"""
    if not isinstance(raw, str):
        raise ValueError(f"清单摘要的 sha256 不是字符串: {raw!r}")
    value = raw.strip().lower()
    if len(value) != 64 or not all(c in _HEX_DIGITS for c in value):
        raise ValueError(f"清单摘要的 sha256 不是 64 位十六进制: {raw!r}")
    return value


@dataclass(frozen=True, slots=True)
class ManifestDigest:
    """单个源清单（latest-{source}.json）的内容摘要"""

    sha256: str
    # base64(minisign 签名文本)；缺省 = 只验完整性。索引自身被签名之前它**不是信任锚**
    signature: str | None = None

    @classmethod
    def from_dict(cls, data: Mapping[str, object]) -> "ManifestDigest":
        """从版本索引条目的 manifests.<source> 构造"""
        if not isinstance(data, Mapping):
            raise ValueError(f"清单摘要不是对象: {data!r}")

        raw_signature = data.get("signature")
        if raw_signature is not None and not isinstance(raw_signature, str):
            raise ValueError(f"清单摘要的 signature 不是字符串: {raw_signature!r}")
        signature = raw_signature.strip() if isinstance(raw_signature, str) else ""
        return cls(sha256=_parse_sha256(data.get("sha256")), signature=signature or None)

    def to_dict(self) -> dict[str, str]:
        """转为版本索引条目里的 manifests.<source> 对象（键序 sha256 → signature）"""
        payload = {"sha256": self.sha256}
        if self.signature:
            payload["signature"] = self.signature
        return payload


def _parse_manifests(raw: object) -> dict[Source, ManifestDigest]:
    """解析 manifests：仅接受三个已知源，未知源直接报错（静默丢弃等于让摘要凭空消失）"""
    if raw is None:
        return {}
    if not isinstance(raw, Mapping):
        raise ValueError(f"manifests 不是对象: {raw!r}")

    known = {source.value for source in _MANIFEST_ORDER}
    unknown = [key for key in raw if key not in known]
    if unknown:
        raise ValueError(
            f"manifests 出现未知源 {join_repr(unknown)}，可选值为 {join_repr(s.value for s in _MANIFEST_ORDER)}"
        )

    manifests: dict[Source, ManifestDigest] = {}
    for source in _MANIFEST_ORDER:
        item = raw.get(source.value)
        if item is None:
            continue
        try:
            manifests[source] = ManifestDigest.from_dict(item)
        except ValueError as e:
            raise ValueError(f"manifests.{source.value} 非法: {e}") from e
    return manifests


@dataclass(frozen=True, slots=True)
class VersionIndex:
    """版本索引条目：versions.json 数组中的一条记录"""

    version: Version
    severity: Severity = field(default_factory=Severity.default)
    # 各源清单的内容摘要；空 = 老索引 / 未回填，客户端对该源走 legacy 路径
    manifests: Mapping[Source, ManifestDigest] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: Mapping[str, object]) -> "VersionIndex":
        """从 versions.json 的单条对象构造

        未知字段一律**报错**而不是忽略：`read → merge → write` 是本索引唯一的写路径，
        忽略等于"回填的字段在下次发布时静默消失"。新增字段时必须同时改这里与 [`to_dict`](VersionIndex.to_dict)。
        """
        if not isinstance(data, Mapping):
            raise ValueError(f"版本索引条目不是对象: {data!r}")

        unknown = [key for key in data if key not in _KNOWN_FIELDS]
        if unknown:
            raise ValueError(
                f"版本索引条目出现未知字段 {join_repr(unknown)}，可选值为 {join_repr(sorted(_KNOWN_FIELDS))}；"
                "新增字段必须同时加进 VersionIndex.from_dict / to_dict，否则 read→write 会静默丢弃"
            )

        raw_version = data.get("version")
        if not isinstance(raw_version, str) or not raw_version.strip():
            raise ValueError("版本索引条目缺少合法的 version")
        try:
            version = Version.parse(raw_version)
        except ValueError as e:
            raise ValueError(f"version={raw_version!r} 非法: {e}") from e

        try:
            manifests = _parse_manifests(data.get("manifests"))
        except ValueError as e:
            raise ValueError(f"{version} 的清单摘要非法: {e}") from e

        raw_severity = data.get("severity")
        if raw_severity is None:
            return cls(version=version, manifests=manifests)
        try:
            severity = Severity(raw_severity)
        except ValueError as e:
            raise ValueError(f"severity {raw_severity!r} 非法，应为 {join_repr(s.value for s in Severity)}") from e
        return cls(version=version, severity=severity, manifests=manifests)

    def to_dict(self) -> dict[str, object]:
        """转为 versions.json 的单条对象（键序 version → severity → manifests）"""
        payload: dict[str, object] = {"version": str(self.version), "severity": str(self.severity)}
        if self.manifests:
            payload["manifests"] = {
                source.value: self.manifests[source].to_dict()
                for source in _MANIFEST_ORDER
                if source in self.manifests
            }
        return payload
