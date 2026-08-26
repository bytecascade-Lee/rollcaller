#!/usr/bin/env python3
"""
CI 构建脚本：在 CI 中构建并打包 Tauri 应用（构建、安装包/便携版与签名收集）。

发布（GitHub Release + CNB Release + 自动更新清单）统一由 scripts/publish.py 负责，
本脚本只负责构建产物。

版本号来源（GitHub Actions 环境变量，由脚本统一提取与校验，而非在 workflow 中
用 PowerShell 重复实现）：
    - workflow_dispatch: 从 inputs.version 读取（INPUT_VERSION）
    - push tag: 从 GITHUB_REF_NAME 读取（自动去掉前导 v）
等级校验（min_level="rc"）：放行 rc 及以上，禁止 alpha/beta。

用法:
    uv run python scripts/release_ci.py build --target <target>
"""

import argparse
import os
from pathlib import Path

import update_version
from common import builder, packager, targets, tauri_cli, version
from common.logger import log

ROOT = Path(__file__).resolve().parent.parent
BACKEND = ROOT / "backend"


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def ci_version(min_level: str = "rc") -> str:
    """
    从 GitHub Actions 环境变量提取并校验发布版本号。

    min_level: CI 发布默认放行 rc 及以上（禁止 alpha/beta）。
    """
    if os.environ.get("GITHUB_EVENT_NAME") == "workflow_dispatch":
        raw = os.environ.get("INPUT_VERSION")
    else:
        raw = os.environ.get("GITHUB_REF_NAME")
    if not raw:
        raise version.VersionNotFoundError("Unable to obtain version, please check the environment variable settings")
    log("INFO", raw)
    try:
        return version.validate(raw, min_level=min_level)
    except version.VersionError as e:
        fail(str(e))


def cmd_build(target: str) -> None:
    if not target:
        fail("build 需要 --target（架构，支持别名或完整三元组）")
    try:
        full_target = targets.to_target(target)
    except targets.TargetError as e:
        fail(str(e))
    release_version = ci_version()
    arch = packager.arch_for_target(full_target)
    log("INFO", f"版本号: {release_version} | arch: {arch} | target: {full_target}")

    # 构建前把 5 个版本文件更新为发布版本，使 tauri.conf.json5 与 tag/input 一致。
    # CI 环境随 job 销毁，无需还原。
    update_version.sync(release_version)

    cli_label, cli_cmd = tauri_cli.resolve(ROOT)
    release_dir = builder.build(
        ROOT,
        BACKEND,
        full_target,
        cli_cmd,
        cli_label,
        # BRANCH_NAME/VERSION 被 backend/build.rs 读取并嵌入二进制
        env_overrides={"VERSION": release_version, "BRANCH_NAME": "master"},
    )
    # CI 产物直接输出到工作区根目录，供 upload-artifact 收集
    packager.package_setup(release_dir, release_version, arch, ROOT)
    packager.package_portable(release_dir, release_version, arch, ROOT)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="CI 构建脚本：构建并打包（matrix 中每个架构各跑一次）"
    )
    sub = parser.add_subparsers(dest="command", required=True)
    build_p = sub.add_parser("build", help="构建并打包（matrix 中每个架构各跑一次）")
    build_p.add_argument(
        "--target", required=True,
        help="架构：完整三元组或别名，如 x86_64-pc-windows-msvc / arm64",
    )

    args = parser.parse_args()
    if args.command == "build":
        cmd_build(args.target)


if __name__ == "__main__":
    main()
