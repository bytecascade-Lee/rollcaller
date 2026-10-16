#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
构造并执行 cargo tauri 的常见子命令


CLI 来源：
    1) 仓库内置 paths.TAURI_CLI_PATH，调用形式 `<exe> <命令>`
    2) 系统 `cargo tauri <子命令>`

工作目录恒为 BACKEND_DIR：
    - build：tauri.conf.json5 与 Cargo.toml 均在此处，是命令自身的要求
    - sign ：虽与 cwd 无关，为避免依赖「cargo 派发子命令不需要 manifest」强制使用 BACKEND_DIR

"""

import subprocess
from enum import StrEnum
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple

from common.constant.paths import BACKEND_DIR, TAURI_CLI_PATH
from common.entity.target import Target
from common.logger import info, warning, debug


class TauriError(Exception):
    """tauri-cli 调用失败：二进制不可用，或命令返回非 0"""


class TauriCliSource(StrEnum):
    SYSTEM = "系统 cargo tauri"
    REPOSITORY_BUILT_IN = "仓库内置 tauri-cli"


def _find_cli() -> Tuple[TauriCliSource, List[str]]:
    """从仓库内置与系统两个地方寻找 tauri-cli 二进制"""
    if TAURI_CLI_PATH.is_file():
        info(f"tauri-cli: 仓库内置 {TAURI_CLI_PATH}")
        return TauriCliSource.REPOSITORY_BUILT_IN, [str(TAURI_CLI_PATH)]
    warning(
        f"未找到仓库内置的 tauri-cli（{TAURI_CLI_PATH}），回落到系统 cargo tauri；"
        f"两者可能不是同一版本，发布构建请先备好内置 CLI"
    )
    return TauriCliSource.SYSTEM, ["cargo", "tauri"]


def _run(args: Sequence[str], env: Optional[Dict[str, str]] = None) -> None:
    """在 BACKEND_DIR 下执行一条 tauri 命令；启动失败或退出码非 0 抛 TauriError"""
    _label, prefix = _find_cli()
    cmd = [*prefix, *args]
    debug(f"执行: {' '.join(cmd)} (cwd={BACKEND_DIR})")
    try:
        proc = subprocess.run(cmd, cwd=BACKEND_DIR, env=env)
    except FileNotFoundError as e:
        raise TauriError(f"tauri-cli 不可用: {e}")
    if proc.returncode != 0:
        raise TauriError(f"tauri {' '.join(args)} 失败（退出码 {proc.returncode}）")


def dev(target: Target, wait_frontend: bool = False) -> None:
    """执行 `tauri dev --target <triple>`"""
    args = ["dev", "--target", target.triple]
    if not wait_frontend:
        args.append("--no-dev-server-wait")
    _run(args)


def build(
    target: Target,
    *,
    debug_mode: bool = False,
    bundle: bool = True,
    env: Optional[Dict[str, str]] = None,
) -> None:
    """执行 `tauri build --target <triple>` [--debug --bundle]

    :param target:      待构建的 Target 目标
    :param debug_mode:  是否启用调试模式，启用后产物将生成到 `BACKEND_DIR/target/<triple>/debug` 下
    :param bundle:      是否捆绑产出指定的分发产物
    :param env:         待替换的环境变量，注意需先 os.environ.copy() 后再改
    """
    args = ["build", "--target", target.triple]
    if debug_mode:
        args.append("--debug")
    if not bundle:
        args.append("--no-bundle")
    _run(args, env=env)


def sign(path: Path) -> None:
    """执行 `tauri signer sign <path>`"""
    if not path.is_absolute():
        if not path.is_file():
            raise TauriError(f"待签名文件的路径 {path!r} 既不是绝对路径、也定位不到文件，请传递绝对路径")
        path = path.resolve()
        debug(f"相对路径已按当前工作目录规范化为 {path!r}")
    _run(["signer", "sign", str(path)])
