#!/usr/bin/env python3
"""
tauri-cli 查找模块：确定构建时使用的 tauri-cli 来源。

纯逻辑，无环境嗅探；仅根据文件系统与可执行文件判断。
"""

from pathlib import Path
from typing import List, Tuple

# 候选顺序：工作区根目录 cargo-tauri.exe（CI 由 workflow 下载并缓存）
#           → resources/tauri/cargo-tauri.exe（仓库内置，本地优先）
#           → 系统 cargo tauri（回退）
_CANDIDATES = [
    ("cargo-tauri.exe",),
    ("resources", "tauri", "cargo-tauri.exe"),
]


def resolve(root: Path) -> Tuple[str, List[str]]:
    """
    返回 (显示名, 命令前缀列表)。

    - 找到预编译 exe：返回其路径作为命令前缀
    - 未找到：回退系统 `cargo tauri`
    """
    for rel in _CANDIDATES:
        candidate = root.joinpath(*rel)
        if candidate.exists():
            return str(candidate), [str(candidate)]
    return "cargo tauri (system)", ["cargo", "tauri"]
