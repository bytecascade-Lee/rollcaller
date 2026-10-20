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

import util
from build import builder, hasher, packager, signer, version_files, metadata
from cli import git, rust, tauri
from codegen import CodegenError, generate_all
from common import support
from common.constant.paths import LOCAL_RELEASE_DIR, ASSETS_DIR
from common.entity.target import Target, UnsupportedTargetError
from common.entity.version import Version, VersionError
from common.enums.kind import Kind, KindOSError
from common.enums.mode import Mode
from common.logger import info
from util.sigint import graceful_sigint


class BuildError(Exception):
    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="构建入口：构建 -> 打包 -> 汇总 -> 签名 + sha256")

    parser.add_argument("-v", "--version", metavar="VERSION", help="语义化版本号")
    parser.add_argument("-t", "--target", metavar="TRIPLE", help="完整 Rust 目标三元组")
    parser.add_argument("-m", "--mode", choices=Mode.values(), help="构建模式")
    # [TODO]: parser.add_argument("-b", "--bundle", help="是否捆绑")

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


def main() -> int:
    args = parse_args()

    required_args = (("-v/--version", args.version), ("-t/--target", args.target), ("-m/--mode", args.mode))
    missing = [flag for flag, value in required_args if value is None]
    if missing:
        raise BuildError(f"未传递参数: {', '.join(missing)}")

    version = Version.parse(args.version, erase_build_metadata=True)
    target = Target(rust.resolve_triple(args.target))
    mode = Mode(args.mode)

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
        # develop 更新模式的 debug 构建；macOS 只发 dmg，不做 .app.tar.gz
        if mode == Mode.LOCAL and target.os in Kind.DEVELOP.oses:
            builder.build(target, debug_flag=True, bundle=False, env=None)
        # 4 生成各种包
        artifacts = package(version, target, build_mode=mode)
    finally:
        # 5 还原版本号
        version_files.restore()

    # 6 汇总各种包到统一目录
    collected = collect(artifacts, out_dir)
    # 7 为包进行签名 + 计算 sha256
    sign_and_hash(collected)

    info(f"构建完成: {str(out_dir)!r}")
    return 0


if __name__ == "__main__":
    with graceful_sigint(on_error=util.fail):
        try:
            sys.exit(main())
        except (
                BuildError,
                CodegenError,
                VersionError,
                git.GitError,
                signer.SignError,
                tauri.TauriError,
                packager.PackageError,
                version_files.VersionFileError,
                KindOSError,
                UnsupportedTargetError
        ) as e:
            util.fail(f"构建失败: {e}")
