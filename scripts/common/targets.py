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
ALIASES = {
    "x64": "x86_64-pc-windows-msvc",
    "x86_64": "x86_64-pc-windows-msvc",
    "x86-64": "x86_64-pc-windows-msvc",
    "arm64": "aarch64-pc-windows-msvc",
    "aarch64": "aarch64-pc-windows-msvc",
}


def resolve_target(raw: str) -> str:
    """
    将别名或完整三元组解析为完整 Rust 三元组。

    Raises:
        TargetError: 无法识别的 target
    """
    key = raw.strip().lower()
    if key in ALIASES:
        return ALIASES[key]
    if key in ALL_TARGETS:
        return key
    raise TargetError(
        f"无法识别的 target: {raw!r}，可用: all, {', '.join(ALIASES)}"
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
    return [resolve_target(raw)]
