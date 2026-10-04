#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
语义化版本号
"""

import semver


class Version(semver.Version):
    """语义化版本号：解析容忍前导 v，输出 tag 时携带 v"""

    __slots__ = ()

    @classmethod
    def parse(cls, version: str, optional_minor_and_patch: bool = False) -> "Version":
        version = version.strip().lower()
        if version.startswith("v"):
            version = version[1:]
        return super().parse(version, optional_minor_and_patch=optional_minor_and_patch)

    def is_prerelease(self) -> bool:
        return self.prerelease is not None

    def to_tag(self) -> str:
        return f"v{self}"
