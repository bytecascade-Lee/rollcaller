#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
sha256 封装
"""

import hashlib
from pathlib import Path

from common.logger import success


class Sha256Error(Exception):
    """对文件进行 sha256 计算时出错"""


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
    sha256_file.write_text(h.hexdigest(), encoding="ascii")
    if not sha256_file.read_bytes().strip():
        raise Sha256Error(f"sha256 文件 {str(sha256_file)!r} 内容为空")
    success(f"已生成 sha256 文件 {str(sha256_file)!r}")
    return sha256_file
