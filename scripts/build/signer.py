#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
tauri signer sign 封装

签名密钥统一从环境变量读取，缺失即报错退出：

    TAURI_SIGNING_PRIVATE_KEY           # 私钥内容
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD  # 私钥密码
"""

import os
from pathlib import Path

from cli import tauri
from common.logger import success
from util.str_format import join_repr

KEY_ENV = "TAURI_SIGNING_PRIVATE_KEY"
PASSWORD_ENV = "TAURI_SIGNING_PRIVATE_KEY_PASSWORD"


class SignError(Exception):
    pass


def check_env() -> None:
    """校验签名环境变量齐备"""
    missing = [name for name in (KEY_ENV, PASSWORD_ENV) if not os.environ.get(name)]
    if missing:
        keys = join_repr(missing, sep=" 和 ")
        raise SignError(f"缺少签名环境变量: {keys}，请先在环境中注入")


def sign(file: Path) -> Path:
    """签名并保留伴生文件 `.sig`，返回其路径"""
    signature_file = _sign_to_file(file)
    success(f"已生成签名文件 {str(signature_file)!r}")
    return signature_file


def signature(file: Path) -> str:
    """签名并只返回字符串，立刻删除伴生文件 `.sig`"""
    signature_file = _sign_to_file(file)
    text = signature_file.read_text(encoding="ascii").strip()
    signature_file.unlink(missing_ok=True)
    return text


def _sign_to_file(file: Path) -> Path:
    """调用 tauri 生成伴生 `.sig` 并确认非空"""
    if not file.is_file():
        raise SignError(f"待签名的文件 {str(file)!r} 不存在")
    check_env()
    tauri.sign(file)
    signature_file = file.with_name(file.name + ".sig")
    if not signature_file.is_file():
        raise SignError(f"签名文件 {str(signature_file)!r} 未生成")
    if not signature_file.read_bytes().strip():
        raise SignError(f"签名文件 {str(signature_file)!r} 内容为空")
    return signature_file
