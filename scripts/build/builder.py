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
from common.enums.profile import Profile
from common.logger import info


def release_dir(target: Target, *, profile: Profile = Profile.RELEASE) -> Path:
    """返回 backend/target/<triple>/<profile>"""
    return BACKEND_TARGET_DIR / target.triple / profile.value


def clean_dir(target: Target, *, profile: Profile = Profile.RELEASE) -> None:
    """清掉输出目录下的旧 bundle，避免陈旧安装包的残留"""
    shutil.rmtree(release_dir(target, profile=profile) / "bundle", ignore_errors=True)


def build(target: Target, *, profile: Profile = Profile.RELEASE,
          bundle: bool = True, config: dict | None = None, env: dict[str, str] | None) -> Path:
    """执行 tauri build，返回输出目录

    :param target: 待构建的 Target 目标
    :param profile: Cargo 构建配置；非 release 时以 `--debug` 调用，自定义档位再透传给 cargo
    :param bundle: 是否捆绑产出指定的分发产物
    :param config: 合并进 tauri 配置的覆盖项，序列化后经 `--config` 传入
    :param env: 待替换的环境变量
    """
    clean_dir(target, profile=profile)

    merged = os.environ.copy()
    # CI=1 会让 tauri-cli 的 clap 解析失败
    if (ci := merged.get("CI", "").lower()) not in ("", "true", "false"):
        del merged["CI"]
        info(f"检测到环境变量 CI 值 {ci!r} 非法，已删除")
    if env:
        merged.update(env)

    tauri.build(target, profile=profile, bundle=bundle, config=config, env=merged)
    return release_dir(target, profile=profile)
