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
from build import builder, hasher, packager, signer, version_files
from cli import exec_py, git, rust
from codegen import CODEGEN_FILES
from common.constant.paths import PROJECT_BASE_DIR, LOCAL_RELEASE_DIR, ASSETS_DIR
from common.entity.target import Target
from common.entity.version import Version
from common.enums.kind import Kind
from common.enums.os import OS
from common.logger import info, warning
from util.execute import exe
from util.sigint import graceful_sigint


class BuildError(Exception):
    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="构建入口：构建 → 打包 → 汇总 → 签名 + sha256")

    parser.add_argument("-v", "--version", metavar="VERSION", help="语义化版本号（不允许自带 + 构建元数据）")
    parser.add_argument("-t", "--target", metavar="TRIPLE", help="完整 Rust 目标三元组")
    parser.add_argument("-m", "--mode", choices=("ci", "local"), help="构建模式")
    # [TODO]: parser.add_argument("-b", "--bundle", choices=("ci", "local"), help="是否捆绑")

    return parser.parse_args()


def local_full_version(version: Version) -> Version:
    """本地完整版本号 = 核心版本 + 构建元数据 <branch>.<提交数>.<短哈希>"""
    branch = git.get_branch(cwd=PROJECT_BASE_DIR)
    count = git.get_commit_count(cwd=PROJECT_BASE_DIR)
    short_hash = git.get_head_hash(cwd=PROJECT_BASE_DIR)[:10]
    return Version.parse(f"{version}+{branch}.{count}.{short_hash}")


def codegen() -> None:
    """串行执行全部代码生成脚本"""
    for script in CODEGEN_FILES:
        info(f"执行 {script.name} 生成代码...")
        exec_py.run(script)


def portable_resources(os: OS) -> list[str]:
    return [exe("rollcaller", os),
            "README.md", "LICENSE", "CHANGELOG.md", "RELEASE_NOTES.md",
            "database", "help"
            ]


# [TODO]：后期统一把 Kind 抽取出来，将 `package()` 变为查表 + 分发，以适应跨平台
def package(version: Version, target: Target, build_mode: str) -> list[Path]:
    """生成各种包，返回最终命名的产物路径"""
    artifacts = [
        packager.package_setup(version, target, Kind.NSIS),
        packager.package_zip(portable_resources(target.os), version, target, Kind.PORTABLE),
    ]
    if build_mode == "local":
        artifacts.append(packager.package_zip([exe("rollcaller", target.os)],
                                              version, target, Kind.DEVELOP, debug_mode=True))
    return artifacts


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

    signer.check_env()

    version = Version.parse(args.version)
    if (input_build_info := version.build) is not None:
        version = Version.parse(args.version.split("+", 1)[0])
        warning(f"传递进来的版本号携带有构建信息 {input_build_info!r}，已自动忽略")

    target = Target(rust.resolve_triple(args.target))
    mode = args.mode.lower()

    if mode == "local":
        version = local_full_version(version)
        out_dir = LOCAL_RELEASE_DIR / str(version)
        if not version_files.check_git_status():
            raise BuildError("版本文件或锁文件存在未提交改动，请先提交或清理后再构建")
    else:
        out_dir = ASSETS_DIR

    info(f"版本号: {str(version)!r} | 目标: {target.triple!r} | 模式: {mode!r}")

    try:
        # 1 同步版本号
        version_files.sync(version)
        # 2 代码生成
        codegen()
        # 3 执行构建
        builder.build(target, env=None)
        # develop 更新模式的 debug 构建
        if mode == "local":
            builder.build(target, debug_mode=True, bundle=False, env=None)
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
        except BuildError as e:
            util.fail(f"构建失败: {e}")
