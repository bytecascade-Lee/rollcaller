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

from cli import tauri
from common.logger import success

KEY_ENV = "TAURI_SIGNING_PRIVATE_KEY"
PASSWORD_ENV = "TAURI_SIGNING_PRIVATE_KEY_PASSWORD"


class SignError(Exception):
    pass


def check_env() -> None:
    """校验签名环境变量齐备"""
    missing = [name for name in (KEY_ENV, PASSWORD_ENV) if not os.environ.get(name)]
    if missing:
        keys = " 和 ".join(repr(key) for key in missing)
        raise SignError(f"缺少签名环境变量: {keys}，请先在环境中注入")


def sign(file: Path) -> Path:
    """对文件做 minisign 签名，返回生成的 `.sig` 路径"""
    if not file.is_file():
        raise SignError(f"待签名的文件 {file!r} 不存在")
    check_env()
    tauri.sign(file)
    sig = file.with_name(file.name + ".sig")
    if not sig.is_file():
        raise SignError(f"签名文件 {sig!r} 未生成")
    if not sig.read_bytes().strip():
        raise SignError(f"签名文件 {sig!r} 内容为空")
    success(f"已生成签名文件 {sig}")
    return sig
