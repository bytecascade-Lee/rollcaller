#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
操作系统类型
"""

import os
import sys
from enum import StrEnum


class OSTypeError(Exception):
    """os 无法解析：未识别的操作系统类型"""


class OS(StrEnum):
    """操作系统"""

    WINDOWS = "windows"
    MACOS = "macos"
    LINUX = "linux"

    @classmethod
    def parse(cls, raw: str) -> "OS | None":
        token = raw.strip().lower()
        if token == "windows":
            return cls.WINDOWS
        if token in {"macos", "darwin"}:
            return cls.MACOS
        if token == "linux":
            return cls.LINUX
        return None

    @classmethod
    def current(cls) -> OS:
        """本机操作系统"""
        if os.name == "nt":
            return cls.WINDOWS
        if sys.platform == "darwin":
            return cls.MACOS
        if sys.platform.startswith("linux"):
            return cls.LINUX
        raise OSTypeError(f"无法识别的操作系统: os.name={os.name!r}，sys.platform={sys.platform!r}")

    @property
    def exe_suffix(self) -> str:
        if self == OS.WINDOWS:
            return ".exe"
        return ""
