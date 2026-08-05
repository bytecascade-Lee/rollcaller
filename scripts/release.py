#!/usr/bin/env python3
"""本地构建打包脚本：构建 Tauri 应用并生成 setup 安装包与便携版 zip。

用法:
    python scripts/release.py <版本号>

版本号可带 v 也可不带，例如: v0.1.0-beta.2 或 0.1.0-rc.1
与 CI 发布流程一致，但本地构建对 alpha/beta 不做限制。

产物输出到 <项目根>/release/local/<版本号>+<分支名>.<提交数>.<短哈希>/
    - rollcaller-<版本号>+<分支名>.<提交数>.<短哈希>-windows-x86_64-setup.exe
    - rollcaller-<版本号>+<分支名>.<提交数>.<短哈希>-windows-x86_64-portable.zip
"""

import argparse
import os
import platform
import re
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BACKEND = ROOT / "backend"
FRONTEND = ROOT / "frontend"
CARGO_RELEASE = BACKEND / "target" / "release"
OUTPUT_ROOT = ROOT / "release" / "local"

VERSION_PATTERN = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$")


def fail(message: str) -> None:
    """打印错误信息并以非零退出码结束脚本。"""
    print(f"[错误] {message}", file=sys.stderr)
    raise SystemExit(1)


def ensure_windows() -> None:
    """仅支持 Windows x86_64 开发机，其余平台直接报错退出。"""
    if not sys.platform.startswith("win"):
        fail(f"不支持当前操作系统: {sys.platform}，本地构建仅支持 Windows")
    machine = platform.machine().lower()
    if machine not in ("amd64", "x86_64"):
        fail(f"不支持的 CPU 架构: {machine}，本地构建仅支持 x86_64")


def normalize_version(raw: str) -> str:
    """去除可选的 v 前缀并校验版本号格式（不限制 alpha/beta）。"""
    version = raw.strip()
    if version.startswith("v"):
        version = version[1:]
    if not VERSION_PATTERN.fullmatch(version):
        fail(f"非法版本号: {raw!r}，应为 0.1.0 或 0.1.0-rc.1 等格式")
    return version


def run_git(args: list[str]) -> str:
    """在项目根目录执行 git 命令，失败即退出。"""
    result = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True)
    if result.returncode != 0:
        fail(f"git {' '.join(args)} 失败: {result.stderr.strip()}")
    return result.stdout.strip()


def get_build_info() -> str:
    """构建信息: <分支名>.<HEAD 总提交数>.<短哈希>，分支名中的 / 替换为 -。"""
    branch = run_git(["rev-parse", "--abbrev-ref", "HEAD"]).replace("/", "-")
    count = run_git(["rev-list", "--count", "HEAD"])
    short_hash = run_git(["rev-parse", "--short", "HEAD"])
    return f"{branch}.{count}.{short_hash}"


def cargo_tauri_build() -> None:
    """清理旧 bundle 后执行 cargo tauri build。"""
    if not (FRONTEND / "node_modules").exists():
        fail("frontend/node_modules 不存在，请先执行: cd frontend && pnpm install")
    # tauri-cli 会把 CI 环境变量当作 --ci 参数的默认值，值为非 true/false（如 CI=1）时 clap 解析失败
    env = os.environ.copy()
    if env.get("CI", "").lower() not in ("", "true", "false"):
        del env["CI"]
    # 清掉旧 bundle，保证 nsis 目录下只有一个安装包
    shutil.rmtree(CARGO_RELEASE / "bundle", ignore_errors=True)
    print(">> 正在执行 cargo tauri build ...")
    proc = subprocess.run(["cargo", "tauri", "build"], cwd=BACKEND, env=env)
    if proc.returncode != 0:
        fail(f"cargo tauri build 失败，退出码 {proc.returncode}")


def package_setup(full_version: str, out_dir: Path) -> Path:
    """重命名 bundle/nsis 下唯一的安装包为 setup.exe。"""
    nsis_dir = CARGO_RELEASE / "bundle" / "nsis"
    setups = list(nsis_dir.glob("*.exe"))
    if len(setups) != 1:
        names = [p.name for p in setups]
        fail(f"bundle/nsis 下应恰好有一个安装包，实际有 {len(setups)} 个: {names}")
    dest = out_dir / f"rollcaller-{full_version}-windows-x86_64-setup.exe"
    shutil.copy2(setups[0], dest)
    return dest


def package_portable(full_version: str, out_dir: Path) -> Path:
    """将 rollcaller.exe、config、database 与新建的空白 portable.mode 压缩为便携版 zip。"""
    for name in ("rollcaller.exe", "config", "database"):
        if not (CARGO_RELEASE / name).exists():
            fail(f"target/release 下缺少 {name}，请确认构建产物完整")
    # 空白文件 portable.mode：不在构建产物中，必须在此创建后一并打入
    portable_mode = CARGO_RELEASE / "portable.mode"
    portable_mode.write_bytes(b"")
    dest = out_dir / f"rollcaller-{full_version}-windows-x86_64-portable.zip"
    with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as zf:
        zf.write(CARGO_RELEASE / "rollcaller.exe", "rollcaller.exe")
        for folder in ("config", "database"):
            for path in (CARGO_RELEASE / folder).rglob("*"):
                if path.is_file():
                    zf.write(path, path.relative_to(CARGO_RELEASE).as_posix())
        zf.write(portable_mode, "portable.mode")
    return dest


def main() -> None:
    parser = argparse.ArgumentParser(
        description="本地构建 Tauri 应用并打包 setup.exe 与便携版 zip"
    )
    parser.add_argument(
        "version",
        help="版本号，可带 v 也可不带，例如 v0.1.0-beta.2 或 0.1.0-rc.1",
    )
    args = parser.parse_args()

    ensure_windows()
    version = normalize_version(args.version)
    build_info = get_build_info()
    full_version = f"{version}+{build_info}"
    print(f">> 版本号: {version} | 构建信息: {build_info}")

    out_dir = OUTPUT_ROOT / full_version
    out_dir.mkdir(parents=True, exist_ok=True)

    cargo_tauri_build()

    setup = package_setup(full_version, out_dir)
    portable = package_portable(full_version, out_dir)
    for artifact in (setup, portable):
        size_mb = artifact.stat().st_size / 1024 / 1024
        print(f">> 已生成: {artifact} ({size_mb:.1f} MB)")


if __name__ == "__main__":
    main()
