#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
开发环境准备
"""

from pathlib import Path
from types import ModuleType

from common.constant.paths import DEVELOP_DATA_DIR, LOCAL_RELEASE_DIR, CI_CD_RELEASE_DIR
from common.logger import info, success
from util.fs import relative_path
from . import cargo_config, dependencies, repo_config, toolchain, uv_index
from .errors import CLI_ERRORS, EnvError

# setup 需要预建的运行期目录
DIRECTORIES: tuple[Path, ...] = (
    DEVELOP_DATA_DIR,
    LOCAL_RELEASE_DIR,
    CI_CD_RELEASE_DIR,
)

# 顺序即体检顺序与施工顺序
STEPS: tuple[ModuleType, ...] = (
    repo_config,
    cargo_config,
    uv_index,
    dependencies,
)


def check() -> list[str]:
    """只读体检，返回未通过项的描述"""
    return [*toolchain.problems(), *(problem for step in STEPS for problem in _problems_of(step))]


def _problems_of(module: ModuleType) -> list[str]:
    """单域体检失败降级成一条问题"""
    try:
        return module.problems()
    except (EnvError, *CLI_ERRORS) as error:
        return [f"{module.__name__.rpartition('.')[2]} 检查失败: {error}"]


def setup() -> None:
    """把仓库准备到可开发状态: 预建目录、核对工具链、写仓库级配置、下载前后端依赖、修导入钩子"""
    # 必须先建目录: 日志本身写在 data/logs 下，全新克隆里目录不存在时第一条日志就会失败
    _ensure_directories()
    _require_toolchain()
    for step in STEPS:
        step.apply()
    success("环境准备完成，可以执行 `just dev`")


def _ensure_directories() -> None:
    """预建运行期目录：数据目录、本地发布产物、CI 日志归档（均在 .gitignore 中）"""
    for directory in DIRECTORIES:
        directory.mkdir(parents=True, exist_ok=True)
        info(f"目录就绪: {relative_path(directory)}")


def _require_toolchain() -> None:
    """工具链不满足要求就在任何写操作之前失败"""
    problems = toolchain.problems()
    if problems:
        raise EnvError("工具链不满足要求: " + "；".join(problems))
