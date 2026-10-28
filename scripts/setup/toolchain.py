#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
工具链体检
"""

from dataclasses import dataclass
from typing import Callable

from cli import git, just, node, pnpm, rust, uv
from common.entity.version import Version, VersionError
from common.logger import info
from .errors import CLI_ERRORS


@dataclass(frozen=True)
class Requirement:
    """一条工具链要求；`probe` 由对应的 cli 门面提供，只回裸版本号"""

    program: str
    minimum: Version
    probe: Callable[[], str | None]
    hint: str


# 顺序即体检顺序；下限取实际用到的特性所需
REQUIREMENTS: tuple[Requirement, ...] = (
    Requirement("git", Version.parse("2.30.0"), git.version, "https://git-scm.com/downloads"),
    Requirement("uv", Version.parse("0.5.0"), uv.version, "https://docs.astral.sh/uv/getting-started/installation/"),
    Requirement("just", Version.parse("1.0.0"), just.version, "https://just.systems/man/en/installation.html"),
    Requirement("node", Version.parse("22.0.0"), node.version, "https://nodejs.org/"),
    Requirement("pnpm", Version.parse("11.0.0"), pnpm.version, "https://pnpm.io/installation"),
    Requirement("rustc", Version.parse("1.80.0"), rust.rustc_version, "https://rustup.rs/"),
    Requirement("cargo", Version.parse("1.80.0"), rust.cargo_version, "https://rustup.rs/"),
)


def problems() -> list[str]:
    """逐条探测工具链；通过的写 info，不通过的收进返回值"""
    result: list[str] = []
    for requirement in REQUIREMENTS:
        problem = _check(requirement)
        if problem is not None:
            result.append(problem)
    return result


def _check(requirement: Requirement) -> str | None:
    """探测一条并要求达标；返回问题描述，通过则写 info"""
    try:
        current = _parse(requirement.probe())
    except CLI_ERRORS as error:
        return f"{requirement.program} 无法运行: {error}（安装方式: {requirement.hint}）"

    if current is None:
        return f"{requirement.program} 的版本号无法识别，请确认装的是 {requirement.minimum} 或更高版本"
    if current < requirement.minimum:
        return f"{requirement.program} 版本过低: {current} < {requirement.minimum}（安装方式: {requirement.hint}）"
    info(f"{requirement.program} {current}")
    return None


def _parse(text: str | None) -> Version | None:
    """裸版本号 -> Version；缺失或不是合法 semver 都算认不出"""
    if text is None:
        return None
    try:
        return Version.parse(text, optional_minor_and_patch=True)
    except VersionError:
        return None
