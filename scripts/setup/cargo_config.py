#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
cargo 本地配置
"""

import shutil
import tomllib
from pathlib import Path

from common.constant.paths import (
    CARGO_CONFIG_DIR,
    CARGO_CONFIG_PATH,
    CARGO_CONFIG_TEMPLATE_PATH,
    FRONTEND_TYPES_DIR,
)
from common.logger import info
from util.fs import relative_path
from .errors import EnvError

EXPORT_KEY = "TS_RS_EXPORT_DIR"
DATABASE_KEY = "DEVELOP_DATABASE_FILENAME"


def problems() -> list[str]:
    """检查 backend/.cargo/config.toml 是否就位，且两项关键配置指向正确位置"""
    label = relative_path(CARGO_CONFIG_PATH)
    if not CARGO_CONFIG_PATH.is_file():
        return [f"cargo 本地配置缺失: {label}；执行 `just setup` 从模板复制"]

    try:
        data = tomllib.loads(CARGO_CONFIG_PATH.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        return [f"读取 {label} 失败: {error}"]

    variables = data.get("env")
    if not isinstance(variables, dict):
        return [f"{label} 缺少 [env] 段；删除该文件后执行 `just setup` 可重新生成"]

    export_dir = _export_dir(variables.get(EXPORT_KEY))
    if export_dir is None:
        return [f"{label} 的 {EXPORT_KEY} 缺失或写法无法识别，可参照 "
                f"{relative_path(CARGO_CONFIG_TEMPLATE_PATH)} 里的写法"]
    if export_dir != FRONTEND_TYPES_DIR.resolve():
        return [f"{label} 的 {EXPORT_KEY} 指向 {str(export_dir)!r}，"
                f"应为 {relative_path(FRONTEND_TYPES_DIR)!r}；否则类型会被导出到别处"]

    database = variables.get(DATABASE_KEY)
    if not isinstance(database, str) or not database.strip():
        return [f"{label} 的 {DATABASE_KEY} 为空，开发模式启动时会断言失败"]

    info(f"cargo 本地配置就位: {label}")
    return []


def apply() -> None:
    """缺失时从模板原样复制；已存在时跳过，避免覆盖本机的私有配置"""
    if CARGO_CONFIG_PATH.is_file():
        info(f"cargo 本地配置已存在，跳过: {relative_path(CARGO_CONFIG_PATH)}")
        return
    try:
        CARGO_CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(CARGO_CONFIG_TEMPLATE_PATH, CARGO_CONFIG_PATH)
    except OSError as error:
        raise EnvError(f"复制 cargo 本地配置失败: {error}") from error
    info(f"已从模板生成 cargo 本地配置: {relative_path(CARGO_CONFIG_PATH)}")


def _export_dir(value: object) -> Path | None:
    """解出 env 里的路径项：支持 {value, relative = true} 与绝对路径字符串，其余一律视为无效"""
    relative = isinstance(value, dict) and bool(value.get("relative"))
    raw = value.get("value") if isinstance(value, dict) else value
    if not isinstance(raw, str):
        return None
    # relative = true 的锚点是 BACKEND_DIR
    path = CARGO_CONFIG_DIR.parent / raw if relative else Path(raw)
    return path.resolve() if path.is_absolute() else None
