#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
语义化版本号
"""

import re
from typing import override

import semver


class VersionError(Exception):
    pass


class Version(semver.Version):
    """语义化版本号：解析容忍前导 v，输出 tag 时携带 v"""

    __slots__ = ()

    @classmethod
    @override
    def parse(cls, version: str, optional_minor_and_patch: bool = False, erase_build_metadata: bool = False) -> "Version":
        s = version.strip().lower()
        if s.startswith("v"):
            s = s[1:]

        base, sep, build = s.partition("+")

        base = cls.sanitize_base_partition(base)

        if sep and (not erase_build_metadata):
            s = f"{base}+{cls.sanitize_build_partition(build)}"
        else:
            s = base

        try:
            return super().parse(s, optional_minor_and_patch=optional_minor_and_patch)
        except (ValueError, TypeError) as e:
            raise VersionError(f"版本号 {version!r} 格式非法: {e}") from e

    def is_prerelease(self) -> bool:
        return self.prerelease is not None

    def to_tag(self) -> str:
        return f"v{self}"

    @staticmethod
    def sanitize_build_partition(value: str) -> str:
        """
        把任意字符串规范成合法的 SemVer 构建元数据片段

        - 连续非法字符转为 `-`
        - 连续 `-`/`.` 折叠为 `-`
        - 去首尾 `-`/`.`
        """
        s = value.strip().lower()
        s = re.compile(r"[^0-9a-z.-]+").sub("-", s)
        s = re.compile(r"[-.]{2,}").sub("-", s)
        s = re.compile(r"^[-.]+|[-.]+$").sub("", s)
        return s

    @staticmethod
    def sanitize_base_partition(value: str) -> str:
        return value.strip().lower().replace("/", "-").replace("_", "-")
