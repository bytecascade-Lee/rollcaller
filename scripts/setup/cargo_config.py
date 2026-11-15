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
    TS_RS_PATHS,
)
from common.logger import info
from util.fs import relative_path

from .errors import EnvError
from .expectation import Expectation, Relation

EXPORT_KEY = "TS_RS_EXPORT_DIR"
DATABASE_KEY = "DEVELOP_DATABASE_FILENAME"

LABEL = relative_path(CARGO_CONFIG_PATH)

# 后半段的两条值检查，前面几个前置门通过后才有可比性
EXPORT_EXPECTATION = Expectation(
    EXPORT_KEY,
    f"{LABEL} 的 {EXPORT_KEY}",
    relative_path(TS_RS_PATHS),
    advice=f"配置里应写 '../{relative_path(TS_RS_PATHS)}'，否则类型会被导出到其他地方",
)

DATABASE_EXPECTATION = Expectation(
    DATABASE_KEY,
    f"{LABEL} 的 {DATABASE_KEY}",
    None,
    relation=Relation.EXISTS,
    advice="开发模式启动时会断言失败，请在 [env] 段补一个非空值",
)


def problems() -> list[str]:
    """检查 backend/.cargo/config.toml 是否就位，且两项关键配置指向正确位置"""
    if not CARGO_CONFIG_PATH.is_file():
        return [f"cargo 本地配置缺失: {LABEL}；执行 `just setup` 从模板复制"]

    try:
        data = tomllib.loads(CARGO_CONFIG_PATH.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        return [f"读取 {LABEL} 失败: {error}"]

    variables = data.get("env")
    if not isinstance(variables, dict):
        return [f"{LABEL} 缺少 [env] 段；删除该文件后执行 `just setup` 可重新生成"]

    export_dir = _export_dir(variables.get(EXPORT_KEY))
    if export_dir is None:
        return [
            f"{LABEL} 的 {EXPORT_KEY} 缺失或写法无法识别，可参照 {relative_path(CARGO_CONFIG_TEMPLATE_PATH)} 里的写法"
        ]

    result = _mismatches(export_dir, variables.get(DATABASE_KEY))
    if result:
        return result
    info(f"cargo 本地配置就位: {LABEL}")
    return []


def _mismatches(export_dir: Path, database: object) -> list[str]:
    """两条值检查各判一次；老写法是早退，同时坏两项时只会报第一条"""
    rows = (
        (EXPORT_EXPECTATION, relative_path(export_dir)),
        # 只认非空字符串：写成 `{value = ...}` 之类的表同样算没配
        (DATABASE_EXPECTATION, database.strip() if isinstance(database, str) else None),
    )
    result: list[str] = []
    for expectation, actual in rows:
        problem = expectation.judge(actual)
        if problem is not None:
            result.append(problem)
    return result


def apply() -> None:
    """缺失时从模板原样复制；已存在时跳过，避免覆盖本机的私有配置"""
    if CARGO_CONFIG_PATH.is_file():
        info(f"cargo 本地配置已存在，跳过: {LABEL}")
        return
    try:
        CARGO_CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(CARGO_CONFIG_TEMPLATE_PATH, CARGO_CONFIG_PATH)
    except OSError as error:
        raise EnvError(f"复制 cargo 本地配置失败: {error}") from error
    info(f"已从模板生成 cargo 本地配置: {LABEL}")


def _export_dir(value: object) -> Path | None:
    """解出 env 里的路径项：支持 {value, relative = true} 与绝对路径字符串，其余一律视为无效"""
    relative = isinstance(value, dict) and bool(value.get("relative"))
    raw = value.get("value") if isinstance(value, dict) else value
    if not isinstance(raw, str):
        return None
    # relative = true 的锚点是 BACKEND_DIR
    path = CARGO_CONFIG_DIR.parent / raw if relative else Path(raw)
    return path.resolve() if path.is_absolute() else None
