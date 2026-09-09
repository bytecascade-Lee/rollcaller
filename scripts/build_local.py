#!/usr/bin/env python3
"""
本地构建打包脚本：构建 Tauri 应用并生成 setup 安装包与便携版 zip（含签名）。

用法:
    uv run python scripts/build_local.py <版本号> [--target <target>] [--output-dir <dir>]

版本号可带 v 也可不带，例如: v0.1.0-beta.2 或 0.1.0-rc.1。
target 支持简化别名与 all（--target all = x86_64 + arm64，缺省为本机默认架构）。

签名（本脚本只管构建，清单由 publish_local.py 负责）：
- setup 安装包的 .sig 由 tauri bundler（createUpdaterArtifacts）构建时自动生成；
- portable zip 打包后调用 tauri signer sign 手动签名，产出 zip.sig；
- develop 直更 zip（debug exe 打成 zip，供 AppMode::Develop 自更新演练）同样
  调用 tauri signer sign 签名，产出 develop.zip.sig；
- 都依赖环境变量 TAURI_SIGNING_PRIVATE_KEY / TAURI_SIGNING_PRIVATE_KEY_PASSWORD
  （缺失即报错退出，见 common/signer.py）。密钥不落代码、不进配置。

版本号处理：构建前自动调用 update_version.py 临时把版本文件与 uv.lock 更新为
传入版本号，构建完成后还原（不提交）。版本文件工作区不干净时直接报错，
避免构建后 git 还原误伤你的改动。

产物输出到 <output-dir>/<版本号>+<分支名>.<提交数>.<短哈希>/
    rollcaller-<版本号>+<构建信息>-windows-<arch>-setup.exe (+ .sig)
    rollcaller-<版本号>+<构建信息>-windows-<arch>-portable.zip (+ .sig)
    rollcaller-<版本号>+<构建信息>-windows-<arch>-develop.zip (+ .sig)  # 本机 arch
"""

import argparse
import os
import platform
import subprocess
import sys
from pathlib import Path

import update_version
from common import builder, git, packager, signer, targets, tauri_cli, version
from common import versions_index
from common.logger import log

ROOT = Path(__file__).resolve().parent.parent
BACKEND = ROOT / "backend"
DEFAULT_OUTPUT = versions_index.default_local_dir(ROOT)

# update_version.py 维护的版本文件 + 锁文件；本地构建后需还原
VERSION_FILES = [
    "pyproject.toml",
    "backend/tauri.conf.json5",
    "backend/Cargo.toml",
    "backend/Cargo.lock",
    "frontend/package.json",
    "uv.lock",
]


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def build_targets(
    target_list,
    release_version: str,
    full_version: str,
    out_dir: Path,
) -> None:
    """对每个 target 依次构建并打包（含 portable zip 手动签名）。"""
    cli_label, cli_cmd = tauri_cli.resolve(ROOT)
    for t in target_list:
        arch = packager.arch_for_target(t)
        release_dir = builder.build(
            ROOT, BACKEND, t, cli_cmd, cli_label,
            # VERSION 被 backend/build.rs 读取并嵌入二进制；签名密钥已由调用方注入环境
            env_overrides={"VERSION": release_version},
        )
        setup = packager.package_setup(release_dir, full_version, arch, out_dir)
        portable = packager.package_portable(release_dir, full_version, arch, out_dir)
        signer.sign_artifact(portable, ROOT)
        for artifact in (setup, portable):
            size_mb = artifact.stat().st_size / 1024 / 1024
            log("INFO", f"已生成: {artifact} ({size_mb:.1f} MB)")


def host_arch() -> str:
    """本机架构标签（develop 直更产物只构建本机架构）。"""
    machine = platform.machine().lower()
    return {"amd64": "x86_64", "x86_64": "x86_64", "arm64": "arm64", "aarch64": "arm64"}.get(machine, machine)


def build_develop(release_version: str, full_version: str, out_dir: Path) -> Path:
    """cargo build(debug) 产出 rollcaller.exe，打包为 develop 直更 zip（含签名）。

    develop 载荷服务于 AppMode::Develop 的直更演练：产物 zip 内为 debug 构建的
    `rollcaller.exe`（zip 内文件名与目标 exe 同名，Go updater 才能按名覆盖写入
    `backend/target/debug/rollcaller.exe`）。

    - 仅构建本机架构（develop 无跨架构需求）；
    - 版本注入与 release 一致：构建前 `update_version.sync` 已临时改写版本文件，
      VERSION env 透传给 build.rs；
    - **前置**：若 `target/debug/rollcaller.exe` 正被运行（cargo tauri dev），
      链接阶段会因文件占用失败——请先停止 dev 实例再执行。
    """
    arch = host_arch()
    env = os.environ.copy()
    env["VERSION"] = release_version
    log("INFO", f"[develop] cargo tauri build --debug 开始（arch={arch}；若 dev 正在运行将链接失败）")
    proc = subprocess.run(["cargo", "tauri", "build", "--debug"], cwd=BACKEND, env=env)
    if proc.returncode != 0:
        fail("cargo tauri build --debug 失败；若 target/debug/rollcaller.exe 正在运行（tauri dev），请先停止")

    debug_dir = BACKEND / "target" / "debug"
    zip_path = packager.package_develop(debug_dir, full_version, arch, out_dir)
    signer.sign_artifact(zip_path, ROOT)
    size_mb = zip_path.stat().st_size / 1024 / 1024
    log("INFO", f"[develop] 已生成直更产物: {zip_path} ({size_mb:.1f} MB)")
    return zip_path


def build(
    version_arg: str,
    target: str = None,
    output_dir: Path = None,
) -> tuple:
    """本地构建打包（供 release_local.py 统筹调用，也可独立运行）。

    Returns:
        (full_version, out_dir)：full_version = `<版本号>+<构建信息>`，
        out_dir 为其产物目录。构建结束版本文件已还原。
    """
    if not sys.platform.startswith("win"):
        fail(f"不支持当前操作系统: {sys.platform}，本地构建仅支持 Windows")

    try:
        release_version = version.validate(version_arg, min_level=None)
    except version.VersionError as e:
        fail(str(e))
    if "+" in release_version:
        fail(
            f"本地构建会在版本号后追加构建信息，不允许版本号自带 + 构建元数据: {version_arg!r}"
        )
    try:
        target_list = targets.targets_for(target)
    except targets.TargetError as e:
        fail(str(e))

    build_info = git.get_build_info(cwd=ROOT)
    full_version = f"{release_version}+{build_info}"
    out_dir = (output_dir or DEFAULT_OUTPUT) / full_version
    log("INFO", f"版本号: {release_version} | 构建信息: {build_info}")
    log("INFO", f"目标架构: {', '.join(t or '(本机默认)' for t in target_list)}")
    log("INFO", f"产物目录: {out_dir}")

    version_files = [ROOT / p for p in VERSION_FILES]
    clean = git.are_clean(version_files, cwd=ROOT)
    if not clean[0]:
        fail(
            f"版本文件{clean[1].replace(chr(10), '、')}存在未提交改动，请先提交或清理后再构建，"
            "避免构建后 git 还原误伤你的改动"
        )

    try:
        # 构建前统一由 update_version.py 更新版本号（不提交）
        update_version.sync(release_version)
        build_targets(target_list, release_version, full_version, out_dir)
        # develop 直更产物（cargo build debug → develop zip + 签名）
        build_develop(release_version, full_version, out_dir)
    finally:
        git.restore_files(version_files, cwd=ROOT)
        log("INFO", "已用 git 还原版本号文件（未提交）")
    return full_version, out_dir


def main() -> None:
    parser = argparse.ArgumentParser(
        description="本地构建 Tauri 应用并打包 setup.exe / 便携版 zip（含签名，不产清单）"
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
    args = parser.parse_args()

    full_version, out_dir = build(args.version, args.target, Path(args.output_dir))
    log("INFO", f"本地构建完成: {out_dir}（{full_version}）")
    log("INFO", f"下一步生成联调清单: uv run python scripts/publish_local.py {args.version}")


if __name__ == "__main__":
    main()
