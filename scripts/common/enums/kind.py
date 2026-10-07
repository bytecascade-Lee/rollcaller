#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
载荷类型
"""
from enum import Enum


class Kind(Enum):
    """更新载荷类型，目前支持 NSIS、PORTABLE、DEVELOP"""

    # Windows
    # MSI = ("setup", "msi")
    # MSIX = ("setup", "msix")
    NSIS = ("setup", "exe")

    # macOS
    # DMG = ("setup", "dmg")

    # Linux
    # DEB = ("setup", "deb")
    # RPM = ("setup", "rpm")
    # APP_IMAGE = ("setup", "AppImage")

    PORTABLE = ("portable", "zip")
    DEVELOP = ("develop", "zip")

    def __init__(self, role: str, extension: str):
        self.role = role
        self.extension = extension

    def __str__(self) -> str:
        return self.name.lower()
