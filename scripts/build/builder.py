#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
tauri build 封装
"""

import os
import shutil
from pathlib import Path

from cli import tauri
from common.constant.paths import BACKEND_TARGET_DIR
from common.entity.target import Target


def release_dir(target: Target, *, debug_mode: bool = False) -> Path:
    """返回 backend/target/<triple>/debug|release"""
    return BACKEND_TARGET_DIR / target.triple / ("debug" if debug_mode else "release")


def clean_dir(target: Target, *, debug_mode: bool = False) -> None:
    """清掉输出目录下的旧 bundle，避免陈旧安装包的残留"""
    shutil.rmtree(release_dir(target, debug_mode=debug_mode) / "bundle", ignore_errors=True)


def build(target: Target, *, debug_mode: bool = False, bundle: bool = True, env: dict[str, str] | None) -> Path:
    """执行 tauri build，返回输出目录"""
    clean_dir(target, debug_mode=debug_mode)

    merged = os.environ.copy()
    # CI=1 会让 tauri-cli 的 clap 解析失败
    if merged.get("CI", "").lower() not in ("", "true", "false"):
        del merged["CI"]
    if env:
        merged.update(env)

    tauri.build(target, debug_mode=debug_mode, bundle=bundle, env=merged)
    return release_dir(target, debug_mode=debug_mode)
