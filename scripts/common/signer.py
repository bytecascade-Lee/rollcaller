#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
tauri signer sign 封装

签名密钥统一从环境变量读取：

    TAURI_SIGNING_PRIVATE_KEY           # 私钥内容
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD  # 私钥密码

缺失即报错退出。

"""

import os
from pathlib import Path

import util
from cli import tauri
from common.logger import info, debug

KEY_ENV = "TAURI_SIGNING_PRIVATE_KEY"
PASSWORD_ENV = "TAURI_SIGNING_PRIVATE_KEY_PASSWORD"


def check_env() -> None:
    """校验签名环境变量齐备"""
    missing = [name for name in (KEY_ENV, PASSWORD_ENV) if not os.environ.get(name, "")]
    if missing:
        util.fail(f"缺少签名环境变量: {', '.join(missing)!r}，请先在环境中注入")


def sign(file: Path) -> Path:
    """对文件做 minisign 签名，返回生成的 `.sig` 路径"""
    if not file.is_file():
        util.fail(f"待签名的文件 {file!r} 不存在")

    debug(f"检查签名所需的环境变量 {KEY_ENV!r} 与 {PASSWORD_ENV!r}")
    check_env()
    debug(f"对文件 {file.name!r} 执行签名")
    tauri.sign(file)
    sig = file.with_name(file.name + ".sig")
    if not sig.is_file():
        util.fail(f"签名 {sig!r} 未生成")
    if not sig.read_bytes().strip():
        util.fail(f"签名 {sig.name!r} 为空")

    info(f"已生成签名 {sig.name!r}")
    return sig
