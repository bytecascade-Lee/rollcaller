#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
sha256 封装
"""

import hashlib
from pathlib import Path


class Sha256Error(Exception):
    pass


def sha256(file: Path, chunk_size: int = 1024 * 1024) -> Path:
    """对文件做 sha256 计算，返回生成的 `.sha256` 路径"""
    if not file.is_file():
        raise Sha256Error(f"待计算的文件 {str(file)!r} 不存在")
    h = hashlib.sha256()
    with open(file, "rb") as f:
        while True:
            chunk = f.read(chunk_size)
            if not chunk:
                break
            h.update(chunk)
    sha256_file = file.with_name(file.name + ".sha256")
    with open(sha256_file, mode="w", encoding="ascii") as f:
        f.write(h.hexdigest())
    return sha256_file
