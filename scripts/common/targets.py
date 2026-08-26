#!/usr/bin/env python3
"""
Target 解析模块：在简化的架构别名 / "all" / 完整 Rust 三元组之间转换。

本地与 CI 共用，避免把三元组硬编码在多处。
"""

from typing import List, Optional


class TargetError(Exception):
    pass


# 完整 Rust 三元组（与 CI matrix 一致）
ALL_TARGETS = (
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
)

# 简化别名 → 完整三元组（不区分大小写）
ALIAS_TO_TARGET = {
    "x64": "x86_64-pc-windows-msvc",
    "x86_64": "x86_64-pc-windows-msvc",
    "x86-64": "x86_64-pc-windows-msvc",
    "amd64": "x86_64-pc-windows-msvc",
    "arm64": "aarch64-pc-windows-msvc",
    "aarch64": "aarch64-pc-windows-msvc",
}

TARGET_TO_ALISE = {
    "aarch64-pc-windows-msvc": "arm64",
    "x86_64-pc-windows-msvc": "x86_64",
}


def to_target(alise: str) -> str:
    """
    将别名或完整三元组解析为完整 Rust 三元组。

    Raises:
        TargetError: 无法识别的 target
    """
    key = alise.strip().lower()
    if key in ALIAS_TO_TARGET:
        return ALIAS_TO_TARGET[key]
    if key in ALL_TARGETS:
        return key
    raise TargetError(
        f"无法识别的 target: {alise!r}，可用: all, {', '.join(ALIAS_TO_TARGET)}"
    )


def to_alise(target: str) -> str:
    """
    将完整三元组解析为标准别名。

    Raises:
        TargetError: 无法转换的 target
    """
    key = target.strip().lower()
    if key in TARGET_TO_ALISE:
        return TARGET_TO_ALISE[key]
    raise TargetError(
        f"无法转换的 target: {target!r}，可用: all, {', '.join(TARGET_TO_ALISE)}"
    )


def targets_for(raw: Optional[str]) -> List[Optional[str]]:
    """
    将 --target 参数展开为构建目标列表。

    - None        → [None]（本机默认）
    - "all"       → 全部支持的架构（顺序构建）
    - 别名/三元组 → 单个目标

    Returns:
        列表元素为完整三元组或 None（本机默认）
    """
    if raw is None:
        return [None]
    if raw.strip().lower() == "all":
        return list(ALL_TARGETS)
    return [to_target(raw)]
