#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
自动更新清单实体
"""
import re
from dataclasses import dataclass
from datetime import datetime, timezone

from common.entity.version import Version
from common.enums.arch import Arch
from common.enums.kind import Kind
from common.enums.os import OS
from common.enums.severity import Severity


class ManifestError(Exception):
    """清单实体不满足契约，如空字段、非法哈希、时间无时区等"""


@dataclass(frozen=True, slots=True)
class Payload:
    """单个下载产物"""

    url: str
    sha256: str
    signature: str
    size: int

    def __post_init__(self) -> None:
        # url
        if not isinstance(self.url, str):
            raise ManifestError(f"url 必须是字符串: {self.url!r}")
        url = self.url.strip()
        if not url.startswith(("http://", "https://")):
            raise ManifestError(f"url 必须以 http(s):// 开头: {url!r}")
        object.__setattr__(self, "url", url)

        # sha256
        if not isinstance(self.sha256, str):
            raise ManifestError(f"sha256 必须是字符串: {self.sha256!r}")
        sha256 = self.sha256.strip().lower()
        if not re.compile(r"^[0-9a-f]{64}$").fullmatch(sha256):
            raise ManifestError(f"sha256 必须是 64 位十六进制: {self.sha256!r}")
        object.__setattr__(self, "sha256", sha256)

        # signature
        if not isinstance(self.signature, str):
            raise ManifestError(f"signature 必须是字符串: {self.signature!r}")
        signature = self.signature.strip()
        if not signature:
            raise ManifestError("signature 不能为空")
        object.__setattr__(self, "signature", signature)

        # size
        if (not isinstance(self.size, int)) or isinstance(self.size, bool) or (self.size <= 0):
            raise ManifestError(f"size 必须是大于 0 的整数: {self.size!r}")


@dataclass(frozen=True, slots=True)
class Manifest:
    """一个版本及其各平台载荷"""

    version: Version
    release_notes: str
    publish_date: datetime
    severity: Severity
    platforms: dict[OS, dict[Arch, dict[Kind, Payload]]]

    def __post_init__(self) -> None:
        # version
        if not isinstance(self.version, Version):
            raise ManifestError(f"version 必须是 Version: {self.version!r}")

        # release_notes
        if not isinstance(self.release_notes, str) or not self.release_notes.strip():
            raise ManifestError("release_notes 必须是非空字符串")

        # publish_date
        if (not isinstance(self.publish_date, datetime)) or (self.publish_date.tzinfo is None):
            raise ManifestError("publish_date 必须是带时区的 datetime")
        object.__setattr__(self, "publish_date", self.publish_date.astimezone(timezone.utc))

        # severity
        if not isinstance(self.severity, Severity):
            raise ManifestError(f"severity 必须是 Severity 枚举类型: {self.severity!r}")

        # platforms
        if not self.platforms:
            raise ManifestError("platforms 不能为空")

        for os, arches in self.platforms.items():
            if not isinstance(os, OS):
                raise ManifestError(f"platforms 的键必须是 OS: {os!r}")
            if not isinstance(arches, dict) or not arches:
                raise ManifestError(f"'platforms[{os.value}]' 必须是非空 dict")
            if not arches:
                raise ManifestError(f"'platforms[{os.value}]' 不能为空")
            for arch, payloads in arches.items():
                if not isinstance(arch, Arch):
                    raise ManifestError(f"'platforms.{os.value}' 的键必须是 Arch: {arch!r}")
                if not payloads:
                    raise ManifestError(f"'platforms.{os.value}.{arch.value}' 不能为空")
                for kind, payload in payloads.items():
                    # kind
                    if not isinstance(kind, Kind):
                        raise ManifestError(f"'platforms.{os.value}.{arch.value}' 的键必须是 Kind: {kind!r}")

                    # payload
                    if not isinstance(payload, Payload):
                        raise ManifestError(f"'platforms.{os.value}.{arch.value}.{kind}' 必须是 Payload: {payload!r}")
