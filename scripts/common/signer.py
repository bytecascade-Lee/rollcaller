#!/usr/bin/env python3
"""
tauri signer sign 封装：对产物（便携版 zip 等）做 minisign 签名，产出 `<file>.sig`。

签名密钥**不通过参数传递**，统一从环境变量读取（与 tauri bundler 构建时自动
签名同源，CI secrets 与本地 shell 注入方式一致）：

    TAURI_SIGNING_PRIVATE_KEY           私钥内容
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD  私钥密码

缺失即报错退出。本地构建可直接复用已泄露并作废的旧密钥做冒烟，正式发布使用
轮换后的新密钥（由调用方在环境中注入，本模块不感知密钥来源与存放位置）。

用法:
    ensure_signing_env()        # 构建入口先校验，缺失提前报错（setup .sig 生成也依赖它）
    sign_artifact(path, root)   # 签名并返回 <path>.sig；失败抛错退出
"""

import os
import subprocess
from pathlib import Path

from common import tauri_cli
from common.logger import log

# 与 tauri bundler / signer 约定一致的环境变量名
KEY_ENV = "TAURI_SIGNING_PRIVATE_KEY"
PASSWORD_ENV = "TAURI_SIGNING_PRIVATE_KEY_PASSWORD"


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def ensure_signing_env() -> None:
    """校验签名环境变量齐备；缺失报错退出。

    构建流程入口应先调用：tauri bundler（createUpdaterArtifacts 生成 setup.exe 的
    .sig）与 signer sign（portable zip 手动签名）都依赖这两个变量。
    """
    missing = [name for name in (KEY_ENV, PASSWORD_ENV) if not os.environ.get(name, "")]
    if missing:
        fail(
            f"缺少签名环境变量: {', '.join(missing)}。\n"
            f"请先在环境中注入 {KEY_ENV} 与 {PASSWORD_ENV}（tauri 自动签名与 signer sign 均需要）"
        )


def sign_artifact(path: Path, root: Path) -> Path:
    """对文件做 minisign 签名（tauri signer sign），返回生成的 `.sig` 路径。

    密钥经环境变量读取（见模块 doc）；CLI 不传 -k/-p，由 signer 自动从 env 取。
    签名输出固定为 `<file>.sig`（同目录）。
    """
    if not path.is_file():
        fail(f"待签名文件不存在: {path}")
    ensure_signing_env()

    cli_label, cli_cmd = tauri_cli.resolve(root)
    cmd = [*cli_cmd, "signer", "sign", str(path)]
    log("INFO", f"使用 tauri-cli: {cli_label}")
    log("INFO", f"执行: {' '.join(cmd)}")
    proc = subprocess.run(cmd)
    if proc.returncode != 0:
        fail(f"tauri signer sign 失败（退出码 {proc.returncode}）: {path.name}")

    sig = path.with_name(path.name + ".sig")
    if not sig.is_file():
        fail(f"签名未生成: {sig}")
    log("INFO", f"已签名: {sig.name}")
    return sig
