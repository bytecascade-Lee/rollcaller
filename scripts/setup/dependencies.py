#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
依赖与脚本导入钩子
"""

import hashlib
import shutil
from pathlib import Path

from cli import cargo, pnpm, uv
from common.constant.paths import (
    FRONTEND_NODE_DIR,
    SITECUSTOMIZE_TEMPLATE_PATH,
    VENV_DIR,
)
from common.logger import info
from util.fs import relative_path

from .errors import EnvError

HOOK_PATH = VENV_DIR / "sitecustomize.py"


def problems() -> list[str]:
    """检查依赖与导入钩子是否就位，且钩子与模板内容一致"""
    result: list[str] = []
    if not FRONTEND_NODE_DIR.is_dir():
        result.append(f"前端依赖未安装: {relative_path(FRONTEND_NODE_DIR)} 不存在；执行 `just setup`")
    if not VENV_DIR.is_dir():
        result.append(f"Python 虚拟环境未创建: {relative_path(VENV_DIR)} 不存在；执行 `just setup`")
    elif not HOOK_PATH.is_file():
        result.append(f"脚本导入钩子缺失: {relative_path(HOOK_PATH)}；执行 `just setup` 重新复制")
    elif _digest(HOOK_PATH) != _digest(SITECUSTOMIZE_TEMPLATE_PATH):
        result.append(f"脚本导入钩子与模板不一致: {relative_path(HOOK_PATH)}；执行 `just setup` 覆盖")
    if not result:
        info(f"依赖与导入钩子就位: {relative_path(FRONTEND_NODE_DIR)}、{relative_path(VENV_DIR)}")
    return result


def apply() -> None:
    """前端 -> 后端 -> Python -> 重建导入钩子；顺序固定，钩子必须落在虚拟环境建好之后"""
    _install_frontend()
    _fetch_backend()
    _sync_python()
    _install_hook()


def _install_frontend() -> None:
    """按 lockfile 安装前端依赖"""
    info("安装前端依赖: pnpm install --frozen-lockfile")
    try:
        pnpm.install()
    except pnpm.PnpmError as error:
        raise EnvError(f"前端依赖安装失败: {error}") from error


def _fetch_backend() -> None:
    """下载 Rust 依赖"""
    info("下载后端依赖: cargo fetch --locked")
    try:
        cargo.fetch()
    except cargo.CargoError as error:
        raise EnvError(f"后端依赖下载失败: {error}") from error


def _sync_python() -> None:
    """按 uv.lock 同步虚拟环境"""
    info("同步 Python 依赖: uv sync --frozen")
    try:
        uv.sync_frozen()
    except uv.UvError as error:
        raise EnvError(f"Python 依赖同步失败: {error}") from error


def _install_hook() -> None:
    """把被追踪的模板复制进虚拟环境；内容一致就不动，模板改了就覆盖"""
    if not VENV_DIR.is_dir():
        raise EnvError(f"虚拟环境目录 {relative_path(VENV_DIR)} 不存在，uv sync 未成功")
    try:
        if HOOK_PATH.is_file() and _digest(HOOK_PATH) == _digest(SITECUSTOMIZE_TEMPLATE_PATH):
            info(f"脚本导入钩子已是最新: {relative_path(HOOK_PATH)}")
            return
        shutil.copyfile(SITECUSTOMIZE_TEMPLATE_PATH, HOOK_PATH)
    except OSError as error:
        raise EnvError(f"复制脚本导入钩子失败: {error}") from error
    info(f"已更新脚本导入钩子: {relative_path(HOOK_PATH)}")


def _digest(path: Path) -> str:
    """文件内容的 sha256；钩子是几百字节的小文本，整读即可"""
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError as error:
        raise EnvError(f"读取 {relative_path(path)} 失败: {error}") from error
