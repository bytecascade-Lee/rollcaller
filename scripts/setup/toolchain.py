#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
工具链体检
"""

from cli import cargo, git, node, pnpm, rustc, uv
from cli.tool import Tool
from common.entity.version import Version, VersionError
from common.logger import info

from .errors import CLI_ERRORS
from .expectation import Expectation, Relation


def _require(tool: Tool, minimum: str) -> Expectation[Tool]:
    """一条工具链要求：程序名与安装提示都由 Tool 自述"""
    return Expectation(tool, tool.program, Version.parse(minimum), relation=Relation.GE, advice=tool.hint)


# 顺序即体检顺序；下限取实际用到的特性所需
REQUIREMENTS: tuple[Expectation[Tool], ...] = (
    _require(git.TOOL, "2.30.0"),
    _require(uv.TOOL, "0.5.0"),
    _require(node.TOOL, "22.0.0"),
    _require(pnpm.TOOL, "11.0.0"),
    _require(rustc.TOOL, "1.80.0"),
    _require(cargo.TOOL, "1.80.0"),
)


def problems() -> list[str]:
    """逐条探测工具链；通过的写 info，不通过的收进返回值"""
    result: list[str] = []
    for expectation in REQUIREMENTS:
        problem = _check(expectation)
        if problem is not None:
            result.append(problem)
    return result


def _check(expectation: Expectation[Tool]) -> str | None:
    """跑一次版本探针再判定；命令根本跑不起来时说明这一点"""
    try:
        current = _parse(expectation.key.version())
    except CLI_ERRORS as error:
        return f"{expectation.label} 无法运行: {error}"

    problem = expectation.judge(current)
    if problem is not None:
        return problem
    info(f"{expectation.label} {current}")
    return None


def _parse(text: str | None) -> Version | None:
    """裸版本号 -> Version；缺失或不是合法 semver 都算认不出"""
    if text is None:
        return None
    try:
        return Version.parse(text, optional_minor_and_patch=True)
    except VersionError:
        return None
