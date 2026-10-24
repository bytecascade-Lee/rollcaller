#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
构造并执行 tauri 的常见子命令
"""

import json
from pathlib import Path
from typing import Sequence

from cli import process
from common.constant.paths import FRONTEND_NODE_DIR, PROJECT_BASE_DIR
from common.entity.target import Target
from common.enums.profile import Profile


class TauriError(Exception):
    """tauri-cli 调用失败：可执行文件不可用，或命令返回非 0"""


BIN_DIR = FRONTEND_NODE_DIR / ".bin"


def _run(args: Sequence[str], env: dict[str, str] | None = None) -> None:
    """在 BACKEND_DIR 下执行一条 tauri 命令；启动失败或退出码非 0 抛出 TauriError"""
    try:
        binary = process.resolve("tauri", search_dir=BIN_DIR)
    except process.ProcessError:
        raise TauriError(f"未在 {str(BIN_DIR)!r} 下找到 tauri-cli；请先在 frontend/ 下执行 pnpm install") from None

    try:
        process.run([binary, *args], cwd=PROJECT_BASE_DIR, env=env, capture=False, timeout=None)
    except process.ProcessError as error:
        raise TauriError(str(error)) from error


def dev(target: Target, wait_frontend: bool = False) -> None:
    """执行 `tauri dev --target <triple>`"""
    args = ["dev", "--target", target.triple]
    if not wait_frontend:
        args.append("--no-dev-server-wait")
    _run(args)


def build(target: Target, *, profile: Profile = Profile.RELEASE,
          bundle: bool = True, config: dict | None = None, env: dict[str, str] | None = None) -> None:
    """执行 `tauri build --target <triple>` [--debug] [--no-bundle] [--config <json>] [-- --profile <profile>]

    :param target: 待构建的 Target 目标
    :param profile: Cargo 构建配置；非 release 时以 `--debug` 调用，自定义档位再透传给 cargo
    :param bundle: 是否捆绑产出指定的分发产物
    :param config: 合并进 tauri 配置的覆盖项，序列化后经 `--config` 传入
    :param env: 待替换的环境变量，注意需先 os.environ.copy() 后再改
    """
    args = ["build", "--target", target.triple]
    # tauri-cli 只区分 release / 非 release 两档，非 release 一律按 --debug 调用
    if profile != Profile.RELEASE:
        args.append("--debug")
    if not bundle:
        args.append("--no-bundle")
    if config:
        args += ["--config", json.dumps(config, separators=(",", ":"))]
    if profile not in (Profile.RELEASE, Profile.DEBUG):
        # 自定义档位只能透传给 cargo；tauri-cli 会解析它来定位产物目录
        args += ["--", "--profile", profile.value]
    _run(args, env=env)


def sign(path: Path) -> None:
    """执行 `tauri signer sign <path>`"""
    if not path.is_absolute():
        if not path.is_file():
            raise TauriError(f"待签名文件的路径 {path!r} 既不是绝对路径、也定位不到文件，请传递绝对路径")
        path = path.resolve()
    _run(["signer", "sign", str(path)])
