#!/usr/bin/env python3
"""
本地构建打包脚本：构建 Tauri 应用并生成 setup 安装包与便携版 zip。

用法:
    uv run python scripts/release_local.py <版本号> [--target <target>] [--output-dir <dir>]

版本号可带 v 也可不带，例如: v0.1.0-beta.2 或 0.1.0-rc.1
与 CI 发布流程一致，但本地构建对 alpha/beta 不做限制，且产物名携带构建信息。

target 支持简化别名与 all：
    --target all                   # 打包全部支持架构（x86_64 + arm64）
    --target x64 / x86_64 / x86-64 # → x86_64-pc-windows-msvc
    --target arm64 / aarch64       # → aarch64-pc-windows-msvc
    （缺省为本机默认架构）

版本号处理：构建前自动调用 update_version.py 临时把 4 个版本文件与 uv.lock 更新为
传入版本号，构建完成后还原（不提交）。工作区对版本文件干净时用 git checkout 还原
（避免换行符问题）；存在未提交改动时回退脚本还原并给出警告。

产物输出到 <output-dir>/<版本号>+<分支名>.<提交数>.<短哈希>/
    - rollcaller-<版本号>+<构建信息>-windows-<arch>-setup.exe
    - rollcaller-<版本号>+<构建信息>-windows-<arch>-portable.zip
"""

import argparse
import sys
from pathlib import Path

from common import builder, git, packager, targets, tauri_cli, version
from common.logger import log

ROOT = Path(__file__).resolve().parent.parent
BACKEND = ROOT / "backend"
DEFAULT_OUTPUT = ROOT / "release" / "local"

# update_version.py 维护的版本文件 + 锁文件；本地构建后需还原
VERSION_FILES = [
    "pyproject.toml",
    "backend/tauri.conf.json5",
    "backend/Cargo.toml",
    "frontend/package.json",
    "uv.lock",
]


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def build_targets(target_list, release_version: str, full_version: str, out_dir: Path) -> None:
    """对每个 target 依次构建并打包。"""
    cli_label, cli_cmd = tauri_cli.resolve(ROOT)
    for t in target_list:
        arch = packager.arch_for_target(t)
        release_dir = builder.build(
            ROOT, BACKEND, t, cli_cmd, cli_label,
            # VERSION 被 backend/build.rs 读取并嵌入二进制
            env_overrides={"VERSION": release_version},
        )
        setup = packager.package_setup(release_dir, full_version, arch, out_dir)
        portable = packager.package_portable(release_dir, full_version, arch, out_dir)
        for artifact in (setup, portable):
            size_mb = artifact.stat().st_size / 1024 / 1024
            log("INFO", f"已生成: {artifact} ({size_mb:.1f} MB)")


def main() -> None:
    if not sys.platform.startswith("win"):
        fail(f"不支持当前操作系统: {sys.platform}，本地构建仅支持 Windows")

    parser = argparse.ArgumentParser(
        description="本地构建 Tauri 应用并打包 setup.exe 与便携版 zip"
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
        help="产物根目录（内部按 full_version 分子目录），默认 release/local",
    )
    args = parser.parse_args()

    try:
        release_version = version.validate(args.version, min_level=None)
    except version.VersionError as e:
        fail(str(e))
    if "+" in release_version:
        fail(
            f"本地构建会在版本号后追加构建信息，不允许版本号自带 + 构建元数据: {args.version!r}"
        )

    try:
        target_list = targets.targets_for(args.target)
    except targets.TargetError as e:
        fail(str(e))

    build_info = git.get_build_info(cwd=ROOT)
    full_version = f"{release_version}+{build_info}"
    out_dir = Path(args.output_dir) / full_version
    log("INFO", f"版本号: {release_version} | 构建信息: {build_info}")
    log("INFO", f"目标架构: {', '.join(t or '(本机默认)' for t in target_list)}")

    version_files = [ROOT / p for p in VERSION_FILES]
    clean = git.are_clean(version_files, cwd=ROOT)
    if not clean[0]:
        fail(
            f"版本文件{clean[1].replace('\n', '、')}存在未提交改动，请先提交或清理后再构建，"
            "避免构建后 git 还原误伤你的改动"
        )

    try:
        # 构建前统一由 update_version.py 更新版本号（不提交）
        builder.update_version_files(ROOT, release_version)
        build_targets(target_list, release_version, full_version, out_dir)
    finally:
        git.restore_files(version_files, cwd=ROOT)
        log("INFO", "已用 git 还原版本号文件（未提交）")


if __name__ == "__main__":
    main()
