#!/usr/bin/env python3
"""
打包模块：setup 安装包重命名 + 便携版 zip 打包。

架构标识映射（Tauri target 三元组 → 产物文件名中的架构后缀）集中在此，
本地与 CI 共用，保证命名一致。
"""

import shutil
import zipfile
from pathlib import Path
from typing import Optional


class PackageError(Exception):
    pass


# Tauri target 三元组 → 产物文件名中的架构标识
_ARCH_MAP = {
    "aarch64-pc-windows-msvc": "arm64",
    "x86_64-pc-windows-msvc": "x86_64",
}


def arch_for_target(target: Optional[str]) -> str:
    """根据 --target 返回产物命名用的架构标识；未指定时视为 x86_64。"""
    if not target:
        return "x86_64"
    if target not in _ARCH_MAP:
        raise PackageError(
            f"不支持的 target: {target!r}，可选: {sorted(_ARCH_MAP)}"
        )
    return _ARCH_MAP[target]


def release_dir(backend: Path, target: Optional[str]) -> Path:
    """根据是否指定 --target 返回对应的 release 目录。"""
    if target:
        return backend / "target" / target / "release"
    return backend / "target" / "release"


def asset_name(version: str, arch: str, kind: str, ext: str) -> str:
    """产物文件名：rollcaller-<version>-windows-<arch>-<kind>.<ext>"""
    return f"rollcaller-{version}-windows-{arch}-{kind}.{ext}"


def package_setup(release_dir_: Path, version: str, arch: str, out_dir: Path) -> Path:
    """将 bundle/nsis 下唯一的安装包重命名并拷贝到 out_dir。"""
    nsis_dir = release_dir_ / "bundle" / "nsis"
    setups = list(nsis_dir.glob("*.exe"))
    if len(setups) != 1:
        names = [p.name for p in setups]
        raise PackageError(
            f"bundle/nsis 下应恰好有一个安装包，实际有 {len(setups)} 个: {names}"
        )
    out_dir.mkdir(parents=True, exist_ok=True)
    dest = out_dir / asset_name(version, arch, "setup", "exe")
    shutil.copy2(setups[0], dest)
    return dest


def package_portable(release_dir_: Path, version: str, arch: str, out_dir: Path) -> Path:
    """将 rollcaller.exe、config、database help、READMEmd、LICENSE、CHANGELOG.md、RELEASE_NOTES.md 和新建的空白 portable.mode 压缩为便携版 zip。"""
    resources_files = ["rollcaller.exe", "README.md", "LICENSE", "CHANGELOG.md", "RELEASE_NOTES.md"]
    resources_folders = ["config", "database", "help"]
    for name in resources_files + resources_folders:
        if not (release_dir_ / name).exists():
            raise PackageError(f"release 目录下缺少 {name}，请确认构建产物完整")
    out_dir.mkdir(parents=True, exist_ok=True)
    dest = out_dir / asset_name(version, arch, "portable", "zip")
    with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as zf:
        for file in resources_files:
            zf.write(release_dir_ / file, file)
        for folder in resources_folders:
            for path in (release_dir_ / folder).rglob("*"):
                if path.is_file():
                    zf.write(path, path.relative_to(release_dir_).as_posix())
        # 空白文件 portable.mode：不在构建产物中，必须创建并打入
        zf.writestr("portable.mode", b"")
    return dest
