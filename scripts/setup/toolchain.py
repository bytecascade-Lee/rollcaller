#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
工具链体检

逐条探测 `--version` 并与下限比较
"""

import re
from dataclasses import dataclass

from cli import process
from common.logger import info
from .errors import EnvError

# 工具 `--version` 输出里的第一段 x.y[.z]
_VERSION_RE = re.compile(r"(\d+(?:\.\d+)+)")

# 单次版本探测的超时（秒）
PROBE_TIMEOUT = 30


@dataclass(frozen=True)
class Requirement:
    """一条工具链要求；``minimum`` 恒为三段，便于与探测结果直接比较"""

    program: str
    minimum: tuple[int, int, int]
    hint: str


# 顺序即体检顺序；下限取实际用到的特性所需
REQUIREMENTS: tuple[Requirement, ...] = (
    Requirement("git", (2, 30, 0), "https://git-scm.com/downloads"),
    Requirement("uv", (0, 5, 0), "https://docs.astral.sh/uv/getting-started/installation/"),
    Requirement("just", (1, 0, 0), "https://just.systems/man/en/installation.html"),
    Requirement("node", (22, 0, 0), "https://nodejs.org/"),
    Requirement("pnpm", (11, 0, 0), "https://pnpm.io/installation"),
    Requirement("rustc", (1, 80, 0), "https://rustup.rs/"),
    Requirement("cargo", (1, 80, 0), "https://rustup.rs/"),
)


def problems() -> list[str]:
    """逐条探测工具链"""
    result: list[str] = []
    for requirement in REQUIREMENTS:
        try:
            info(f"{requirement.program} {_probe(requirement)}")
        except EnvError as error:
            result.append(str(error))
    return result


def _probe(requirement: Requirement) -> str:
    """探测单个工具，返回它自报的版本串"""
    program = requirement.program
    try:
        result = process.run([program, "--version"], timeout=PROBE_TIMEOUT)
    except process.ProcessError as error:
        raise EnvError(f"未找到 {program!r}，请先安装: {requirement.hint}") from error

    output = f"{result.stdout or ''}{result.stderr or ''}".strip()
    version = _parse_version(output)
    if version is None:
        raise EnvError(f"{program!r} 的 --version 输出无法解析: {output!r}")

    if version < requirement.minimum:
        raise EnvError(f"{program!r} 版本过低: {_format_version(version)} < "
                       f"{_format_version(requirement.minimum)}")
    return _format_version(version)


def _parse_version(text: str) -> tuple[int, int, int] | None:
    """从版本输出里取第一段数字并补齐三段"""
    match = _VERSION_RE.search(text)
    if match is None:
        return None
    parts = [int(part) for part in match.group(1).split(".")][:3]
    parts += [0] * (3 - len(parts))
    return parts[0], parts[1], parts[2]


def _format_version(version: tuple[int, ...]) -> str:
    """版本元组 -> `1.96.0`，只取前三段"""
    return ".".join(str(part) for part in version[:3])
