#!/usr/bin/env python3
"""
本地构建打包脚本：构建 Tauri 应用并生成 setup 安装包与便携版 zip。

用法:
    uv run python scripts/release_local.py <版本号> [--target <target>] [--output-dir <dir>]

版本号可带 v 也可不带，例如: v0.1.0-beta.2 或 0.1.0-rc.1
与 CI 发布流程一致，但本地构建对 alpha/beta 不做限制，且产物名携带构建信息。

产物输出到 <output-dir>/<版本号>+<分支名>.<提交数>.<短哈希>/
    - rollcaller-<版本号>+<构建信息>-windows-<arch>-setup.exe
    - rollcaller-<版本号>+<构建信息>-windows-<arch>-portable.zip
"""

import argparse
import sys
from pathlib import Path

from common import builder, git, packager, tauri_cli, version

ROOT = Path(__file__).resolve().parent.parent
BACKEND = ROOT / "backend"
DEFAULT_OUTPUT = ROOT / "release" / "local"


def fail(message: str) -> None:
    print(f"[错误] {message}", file=sys.stderr)
    raise SystemExit(1)


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
        help="Rust target 三元组，如 aarch64-pc-windows-msvc；缺省为本机默认",
    )
    parser.add_argument(
        "--output-dir",
        default=str(DEFAULT_OUTPUT),
        help="产物根目录（内部按 full_version 分子目录），默认 release/local",
    )
    args = parser.parse_args()

    try:
        release_version = version.validate(args.version, strict=False)
    except version.VersionError as e:
        fail(str(e))
    if "+" in release_version:
        fail(
            f"本地构建会在版本号后追加构建信息，不允许版本号自带 + 构建元数据: {args.version!r}"
        )

    build_info = git.get_build_info(cwd=ROOT)
    full_version = f"{release_version}+{build_info}"
    print(f">> 版本号: {release_version} | 构建信息: {build_info}")

    arch = packager.arch_for_target(args.target)
    cli_label, cli_cmd = tauri_cli.resolve(ROOT)
    release_dir = builder.build(
        ROOT,
        BACKEND,
        args.target,
        cli_cmd,
        cli_label,
        env_overrides={"VERSION": release_version},
    )

    out_dir = Path(args.output_dir) / full_version
    setup = packager.package_setup(release_dir, full_version, arch, out_dir)
    portable = packager.package_portable(release_dir, full_version, arch, out_dir)
    for artifact in (setup, portable):
        size_mb = artifact.stat().st_size / 1024 / 1024
        print(f">> 已生成: {artifact} ({size_mb:.1f} MB)")


if __name__ == "__main__":
    main()
