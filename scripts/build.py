#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
构建入口

顺序执行：
    校验 -> 同步版本号 -> 代码生成 -> 构建 -> 生成各种包 ->
    还原版本号 -> 汇总到统一目录 -> 签名 + 计算 sha256

用法:
    uv run ./scripts/build.py -v 0.1.0 -t x86_64-pc-windows-msvc -m local
    uv run ./scripts/build.py --version 0.1.0 --target aarch64-pc-windows-msvc --mode ci
"""

import argparse
import shutil
import sys
from pathlib import Path
from typing import Callable

from build import builder, hasher, packager, signer, version_files, metadata
from cli import git, rust, tauri
from codegen import CodegenError, generate_all
from common import gate, support
from common.constant.paths import LOCAL_RELEASE_DIR, ASSETS_DIR
from common.entity.target import Target, UnsupportedTargetError
from common.entity.version import Version, VersionError
from common.enums.action import Action
from common.enums.kind import Kind, KindOSError
from common.enums.mode import Mode
from common.enums.profile import Profile
from common.logger import error, info
from util.sigint import graceful_sigint, never_interrupted

SKIP_FRONTEND_BUILD = {"build": {"beforeBuildCommand": None}}


class BuildError(Exception):
    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="构建入口：构建 -> 打包 -> 汇总 -> 签名 + sha256")

    parser.add_argument("-v", "--version", required=True, metavar="VERSION", help="语义化版本号")
    parser.add_argument("-t", "--target", required=True, metavar="TRIPLE", help="完整 Rust 目标三元组")
    parser.add_argument("-m", "--mode", required=True, choices=Mode.values(), help="构建模式")

    return parser.parse_args()


def package(version: Version, target: Target, build_mode: Mode) -> list[Path]:
    """按支持表查表，按 Kind 声明顺序产出该目标在指定模式下的载荷"""
    declared = support.kinds(target)
    kinds = [
        kind for kind in Kind
        if kind in declared and (build_mode == Mode.LOCAL or not kind.local_only)
    ]
    return [packager.produce(version, target, kind) for kind in kinds]


def collect(artifacts: list[Path], out_dir: Path) -> list[Path]:
    """把产物移入统一目录，存在则覆盖，返回汇总后的路径"""
    out_dir.mkdir(parents=True, exist_ok=True)
    collected = []
    for artifact in artifacts:
        dest = out_dir / artifact.name
        dest.unlink(missing_ok=True)
        collected.append(Path(shutil.move(str(artifact), str(dest))))
    return collected


def sign_and_hash(artifacts: list[Path]) -> None:
    """对产物逐个签名并计算 sha256"""
    for artifact in artifacts:
        signer.sign(artifact)
        hasher.sha256(artifact)
        info(f"已对 {str(artifact)!r} 签名并计算 sha256")


def main(*, is_interrupted: Callable[[], bool] = never_interrupted) -> int:
    args = parse_args()

    version = Version.parse(args.version, erase_build_metadata=True)
    target = Target(rust.resolve_triple(args.target))
    mode = Mode(args.mode)

    action = Action.RELEASE if mode == Mode.CI else Action.BUILD
    gate.ensure(target, action, host=rust.get_host_target())

    signer.check_env()

    if mode == Mode.LOCAL:
        version = Version.parse(f"{version}+{metadata.build_metadata()}")
        out_dir = LOCAL_RELEASE_DIR / str(version)
        if not version_files.check_git_status():
            raise BuildError("版本文件或锁文件存在未提交改动，请先提交或清理后再构建")
    else:
        out_dir = ASSETS_DIR

    info(f"版本号: {str(version)!r} | 目标三元组: {target.triple!r} | 模式: {mode.value!r}")

    try:
        # 1 同步版本号
        version_files.sync(version)
        # 2 代码生成
        generate_all()
        # 3 执行构建
        builder.build(target, env=None)
        if mode == Mode.LOCAL and target.os in Kind.DEVELOP.oses:
            builder.build(target, profile=Profile.DEVELOP, bundle=False, config=SKIP_FRONTEND_BUILD, env=None)
        # 4 生成各种包
        artifacts = package(version, target, build_mode=mode)
    finally:
        # 5 还原版本号
        version_files.restore()

    # 中断若落在构建 / 打包期间，版本号已还原，不要再汇总与签名（后者会写 assets/）
    # 退出码由 graceful_sigint 统一给出
    if is_interrupted():
        return 0

    # 6 汇总各种包到统一目录
    collected = collect(artifacts, out_dir)
    # 7 为包进行签名 + 计算 sha256
    sign_and_hash(collected)

    info(f"构建完成: {str(out_dir)!r}")
    return 0


if __name__ == "__main__":
    with graceful_sigint() as is_interrupted:
        try:
            sys.exit(main(is_interrupted=is_interrupted))
        except (
                BuildError,
                CodegenError,
                VersionError,
                gate.GateError,
                git.GitError,
                signer.SignError,
                tauri.TauriError,
                packager.PackageError,
                version_files.VersionFileError,
                KindOSError,
                UnsupportedTargetError
        ) as e:
            if not is_interrupted():
                error(f"构建失败: {e}")
                sys.exit(1)
