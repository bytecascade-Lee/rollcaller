#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
CPU 架构类型
"""

from enum import StrEnum


class ArchTypeError(Exception):
    """arch 无法解析：未识别的CPU 架构类型"""
    pass


class Arch(StrEnum):
    """CPU 架构"""

    X86_64 = "x86_64"
    ARM64 = "arm64"

    @classmethod
    def parse(cls, raw: str) -> "Arch | None":
        token = raw.strip().lower()
        if token in {"x86_64", "x64", "amd64"}:
            return cls.X86_64
        if token in {"arm64", "aarch64"}:
            return cls.ARM64
        return None
