#!/usr/bin/env python3
"""
本地构建 + 发布统筹入口：串联 build_local（构建打包）与 publish_local（生成联调清单）。

用法:
    uv run python scripts/release_local.py <版本号> [--target <target>]
        [--output-dir <dir>] [--severity normal|important|critical]
        [--serve-base <url>]

流程:
    1. build_local：构建 Tauri 应用并打包 setup 安装包（.sig 自动）与便携版 zip
       （tauri signer sign 手动签名）；版本号文件临时更新后 git 还原（不提交）；
    2. publish_local：在产物目录生成 latest-develop.json（v2 结构）与 versions.json。

签名密钥从环境变量 TAURI_SIGNING_PRIVATE_KEY / TAURI_SIGNING_PRIVATE_KEY_PASSWORD
读取（缺失报错退出），不落代码与配置文件。正式发布请注入轮换后的新密钥；
已泄露作废的旧密钥可仅用于本地冒烟测试。

联调: `uv run python scripts/serve_local.py <版本号>` 即可本地 serve 产物目录。
"""

import argparse
import sys
from pathlib import Path

from build_local import DEFAULT_OUTPUT, build
from common import version as version_mod, signer
from common.logger import log
from publish_local import DEFAULT_SERVE_BASE, publish

ROOT = Path(__file__).resolve().parent.parent


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def main() -> None:
    if not sys.platform.startswith("win"):
        fail(f"不支持当前操作系统: {sys.platform}，本地构建仅支持 Windows")

    parser = argparse.ArgumentParser(
        description="本地全链路：构建打包（build_local）+ 生成联调清单（publish_local）"
    )
    parser.add_argument(
        "version",
        help="版本号，可带 v 也可不带，例如 v0.1.0-beta.2 或 0.1.0-rc.1",
    )
    parser.add_argument(
        "--target",
        default=None,
        help="架构（别名/all/完整三元组），如 x64、arm64、all；缺省为本机默认",
    )
    parser.add_argument(
        "--output-dir",
        default=str(DEFAULT_OUTPUT),
        help=f"产物根目录（内部按 full_version 分子目录），默认 {DEFAULT_OUTPUT}",
    )
    parser.add_argument(
        "--severity",
        choices=("normal", "important", "critical"),
        default="normal",
        help="当前版本的严重级别（默认 normal；critical 表示强制更新）",
    )
    parser.add_argument(
        "--serve-base",
        default=DEFAULT_SERVE_BASE,
        help=f"清单中产物 URL 的服务器 base（默认 {DEFAULT_SERVE_BASE}）",
    )
    args = parser.parse_args()

    try:
        version_mod.validate(args.version, min_level=None)
    except version_mod.VersionError as e:
        fail(str(e))

    # 0. 签名环境变量缺失时提前报错（tauri 自动签名与 signer sign 都依赖）
    signer.ensure_signing_env()

    # 1. 构建打包（版本文件临时更新并还原；setup/portable 签名）
    full_version, out_dir = build(args.version, args.target, Path(args.output_dir))

    # 2. 生成联调清单（latest-develop.json + versions.json）
    publish(
        args.version,
        out_dir=out_dir,
        severity=args.severity,
        serve_base=args.serve_base,
        local_root=Path(args.output_dir),
    )

    log("INFO", f"本地全链路完成: {full_version}")
    log("INFO", f"产物目录: {out_dir}")
    log("INFO", f"serve 联调: uv run python scripts/serve_local.py {args.version}")


if __name__ == "__main__":
    main()
