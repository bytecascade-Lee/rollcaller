#!/usr/bin/env python3
"""
版本号处理模块：规范化、校验、比较

所有函数均为纯函数，无副作用，无环境嗅探。
"""

import re
from typing import Optional, Tuple

# 语义化版本正则：主版本.次版本.补丁版本(-预发布标识)?(+构建元数据)?
# 预发布标识允许：字母、数字、连字符、点
# 构建元数据允许：字母、数字、连字符、点
VERSION_PATTERN = re.compile(
    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-([0-9A-Za-z.-]+))?"
    r"(?:\+([0-9A-Za-z.-]+))?$"
)


class VersionError(Exception):
    """版本号相关错误的基类"""
    pass


class InvalidVersionError(VersionError):
    """版本号格式非法"""
    pass


class PrereleaseDisallowedError(VersionError):
    """预发布版本不被允许（strict模式）"""
    pass


def normalize(raw: str) -> str:
    """
    规范化版本号：去除前导 'v'，保留其余部分原样

    >>> normalize("v0.1.0")
    "0.1.0"
    >>> normalize("0.1.0-rc.1")
    "0.1.0-rc.1"
    >>> normalize("v0.1.0+abc.123")
    "0.1.0+abc.123"
    """
    version = raw.strip()
    if version.startswith("v"):
        version = version[1:]
    return version


def validate(version: str, strict: bool = False) -> str:
    """
    校验版本号格式，返回规范化后的版本号

    Args:
        version: 待校验的版本号字符串
        strict: True 时禁止包含 alpha/beta 预发布标识（rc 等仍允许，与 CI 一致）

    Returns:
        校验通过后的版本号（去除前导v）

    Raises:
        InvalidVersionError: 格式非法
        PrereleaseDisallowedError: strict模式且包含 alpha/beta 预发布标识
    """
    cleaned = normalize(version)

    match = VERSION_PATTERN.fullmatch(cleaned)
    if not match:
        raise InvalidVersionError(
            f"非法版本号: {version!r}，"
            f"应符合语义化版本规范，例如 0.1.0 或 0.1.0-rc.1"
        )

    major, minor, patch, prerelease, build = match.groups()

    if strict and prerelease is not None:
        # 检测是否包含 alpha/beta（不区分大小写）
        if re.search(r'(?i)alpha|beta', prerelease):
            raise PrereleaseDisallowedError(
                f"strict 模式下不允许预发布版本: {version!r} "
                f"(包含 alpha 或 beta)"
            )

    return cleaned


def parse(version: str) -> Tuple[int, int, int, Optional[str], Optional[str]]:
    """
    解析版本号为结构化数据

    Returns:
        (major, minor, patch, prerelease, build)
        其中 prerelease 和 build 可能为 None
    """
    cleaned = normalize(version)
    match = VERSION_PATTERN.fullmatch(cleaned)
    if not match:
        raise InvalidVersionError(f"无法解析版本号: {version!r}")
    major, minor, patch, prerelease, build = match.groups()
    return int(major), int(minor), int(patch), prerelease, build


def is_prerelease(version: str) -> bool:
    """判断是否为预发布版本（包含 -alpha / -beta / -rc 等）"""
    _, _, _, prerelease, _ = parse(version)
    return prerelease is not None


def bump_patch(version: str) -> str:
    """
    将补丁版本号 +1，保留预发布标识和构建元数据

    >>> bump_patch("0.1.0")
    "0.1.1"
    >>> bump_patch("0.1.0-rc.1")
    "0.1.1-rc.1"
    """
    major, minor, patch, prerelease, build = parse(version)
    new_patch = patch + 1
    result = f"{major}.{minor}.{new_patch}"
    if prerelease:
        result += f"-{prerelease}"
    if build:
        result += f"+{build}"
    return result


def bump_minor(version: str) -> str:
    """将次版本号 +1，补丁归零，保留预发布标识和构建元数据"""
    major, minor, _, prerelease, build = parse(version)
    new_minor = minor + 1
    result = f"{major}.{new_minor}.0"
    if prerelease:
        result += f"-{prerelease}"
    if build:
        result += f"+{build}"
    return result


def bump_major(version: str) -> str:
    """将主版本号 +1，次版本和补丁归零，保留预发布标识和构建元数据"""
    major, _, _, prerelease, build = parse(version)
    new_major = major + 1
    result = f"{new_major}.0.0"
    if prerelease:
        result += f"-{prerelease}"
    if build:
        result += f"+{build}"
    return result


def strip_prerelease(version: str) -> str:
    """
    去除预发布标识和构建元数据，只保留核心版本号

    >>> strip_prerelease("0.1.0-rc.1+sha.123")
    "0.1.0"
    """
    major, minor, patch, _, _ = parse(version)
    return f"{major}.{minor}.{patch}"
