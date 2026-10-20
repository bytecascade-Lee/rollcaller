#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
构造并执行 tauri 的常见子命令
"""

from pathlib import Path
from typing import Sequence

from cli import process
from common.constant.paths import BACKEND_DIR, FRONTEND_NODE_DIR
from common.entity.target import Target


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
        # 构建输出要实时可见，不捕获；构建可能很久，不设超时
        process.run([binary, *args], cwd=BACKEND_DIR, env=env, capture=False, timeout=None)
    except process.ProcessError as error:
        raise TauriError(str(error)) from error


def dev(target: Target, wait_frontend: bool = False) -> None:
    """执行 `tauri dev --target <triple>`"""
    args = ["dev", "--target", target.triple]
    if not wait_frontend:
        args.append("--no-dev-server-wait")
    _run(args)


def build(target: Target, *, debug_flag: bool = False, bundle: bool = True, env: dict[str, str] | None = None) -> None:
    """执行 `tauri build --target <triple>` [--debug --bundle]

    :param target:      待构建的 Target 目标
    :param debug_flag:  是否启用调试模式，启用后产物将生成到 `BACKEND_DIR/target/<triple>/debug` 下
    :param bundle:      是否捆绑产出指定的分发产物
    :param env:         待替换的环境变量，注意需先 os.environ.copy() 后再改
    """
    args = ["build", "--target", target.triple]
    if debug_flag:
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
    _run(["signer", "sign", str(path)])
