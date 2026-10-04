#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
操作系统类型
"""

from enum import StrEnum


class OSTypeError(Exception):
    """os 无法解析：未识别的操作系统类型"""
    pass


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
