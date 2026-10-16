#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
载荷类型
"""

from enum import Enum

from common.enums.os import OS


class Kind(Enum):
    """更新载荷类型，目前支持 NSIS、PORTABLE、DEVELOP"""

    # Windows
    # MSI = "setup"
    # MSIX = "setup"
    NSIS = "setup"

    # macOS
    # DMG = "setup"

    # Linux
    # DEB = "setup"
    # RPM = "setup"
    # APP_IMAGE = "setup"

    # Common
    PORTABLE = "portable"
    DEVELOP = "develop"

    def __init__(self, role: str) -> None:
        self.role = role

    def extension_with_dot(self, os: OS) -> str:
        match self:
            case Kind.NSIS:
                extension = "exe"
            case Kind.PORTABLE | Kind.DEVELOP:
                if os == OS.WINDOWS:
                    extension = "zip"
                else:
                    extension = "tar.gz"
            case _ as unreachable:
                raise AssertionError(unreachable)
        return f".{extension}"

    def __str__(self) -> str:
        return self.name.lower()
