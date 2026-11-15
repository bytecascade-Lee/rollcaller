#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
sha256 封装
"""

import hashlib
from pathlib import Path

from common.logger import success

CHUNK_SIZE = 1024 * 1024


class Sha256Error(Exception):
    """对文件进行 sha256 计算时出错"""


def digest(file: Path, chunk_size: int = CHUNK_SIZE) -> str:
    """计算文件的 sha256 十六进制字符串"""
    if not file.is_file():
        raise Sha256Error(f"待计算的文件 {str(file)!r} 不存在")
    hasher = hashlib.sha256()
    with open(file, "rb") as f:
        while chunk := f.read(chunk_size):
            hasher.update(chunk)
    return hasher.hexdigest()


def sha256(file: Path, chunk_size: int = CHUNK_SIZE) -> Path:
    """计算文件的 sha256 并生成伴生文件 `.sha256`，返回其路径"""
    sha256_file = file.with_name(file.name + ".sha256")
    sha256_file.write_text(digest(file, chunk_size), encoding="ascii")
    if not sha256_file.read_bytes().strip():
        raise Sha256Error(f"sha256 文件 {str(sha256_file)!r} 内容为空")
    success(f"已生成 sha256 文件 {str(sha256_file)!r}")
    return sha256_file
