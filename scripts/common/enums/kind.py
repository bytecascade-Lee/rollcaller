#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
载荷类型
"""

from collections.abc import Mapping
from enum import Enum, StrEnum
from types import MappingProxyType

from common.enums.os import OS
from util.str_format import join_repr


class KindOSError(Exception):
    """载荷类型不支持目标操作系统"""


class KindStyle(StrEnum):
    """载荷的产出方式"""

    SETUP = "setup"
    PORTABLE = "portable"
    DEVELOP = "develop"


class Kind(Enum):
    """更新载荷类型"""

    NSIS = (KindStyle.SETUP, ((OS.WINDOWS, ".exe"),))

    DMG = (KindStyle.SETUP, ((OS.MACOS, ".dmg"),))

    DEB = (KindStyle.SETUP, ((OS.LINUX, ".deb"),))
    APPIMAGE = (KindStyle.SETUP, ((OS.LINUX, ".AppImage"),))

    PORTABLE = (KindStyle.PORTABLE, ((OS.WINDOWS, ".zip"),))
    DEVELOP = (KindStyle.DEVELOP, ((OS.WINDOWS, ".zip"), (OS.LINUX, ".tar.gz")))

    def __init__(self, style: KindStyle, extensions: tuple[tuple[OS, str], ...]) -> None:
        self.key = self.name.lower()
        self.role = style.value.lower()
        self.style = style
        self.bundle = self.name.lower() if style is KindStyle.SETUP else None
        self.extensions: Mapping[OS, str] = MappingProxyType(dict(extensions))
        self.oses = frozenset(self.extensions)

    @property
    def local_only(self) -> bool:
        """是否只用于本地调试，不参与发布"""
        return self.style is KindStyle.DEVELOP

    def suffix(self, os: OS) -> str:
        """该载荷在指定系统上的扩展名，系统不匹配时抛 KindOSError"""
        if os not in self.oses:
            raise KindOSError(
                f"载荷 {self.key!r} 不支持 {os.value!r}，当前仅支持 {join_repr(o.value for o in self.oses)}")
        return self.extensions[os]

    def __str__(self) -> str:
        return self.key
