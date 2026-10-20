#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
将生成的产物进行打包

定位 / 改名交给 Kind 的 bundle + produced_glob；
自打包形态的「根目录 + 成员」放在本模块的配方表里，避免 Kind 同时承担两种语义。
"""

import tarfile
import zipfile
from pathlib import Path
from typing import Iterator, Sequence

from build.builder import release_dir
from common.constant.paths import RESOURCES_FILES
from common.entity.target import Target
from common.entity.version import Version
from common.enums.kind import Kind, KindStyle
from common.enums.os import OS


class PackageError(Exception):
    """打包失败"""


def asset_name(version: Version, target: Target, kind: Kind) -> str:
    """产物文件名：rollcaller-<version>-<os>-<arch>-<role><extension>"""
    return f"rollcaller-{version}-{target.os}-{target.arch}-{kind.role}{kind.suffix(target.os)}"


def produce(version: Version, target: Target, kind: Kind) -> Path:
    """按载荷样式产出最终文件"""
    if kind.style is KindStyle.SETUP:
        return _produce_setup(version, target, kind)
    if kind.style in (KindStyle.PORTABLE, KindStyle.DEVELOP):
        base_dir, resources = _archive_recipe(target, kind)
        return _produce_archive(base_dir, resources, version, target, kind)
    raise PackageError(f"未知的载荷样式: {kind.style!r}")


def _produce_setup(version: Version, target: Target, kind: Kind) -> Path:
    """把 tauri bundler 已产出的安装包重命名为最终产物名"""
    if kind.bundle is None:
        raise PackageError(f"载荷 {kind!s} 未声明 bundle，无法定位安装包")

    setup_dir = release_dir(target) / "bundle" / kind.bundle
    if not setup_dir.is_dir():
        raise PackageError(f"{str(setup_dir)!r} 不存在或不是一个目录")

    setups = [p for p in setup_dir.iterdir()
              if p.is_file() and not p.name.startswith(".") and p.name.endswith(kind.suffix(target.os))]
    if len(setups) != 1:
        names = [repr(path.name) for path in setups]
        raise PackageError(f"{str(setup_dir)!r} 下应恰好有一个 bundler 产物，但实际有 {len(setups)} 个，分别为 {names!r}")
    dest = setup_dir / asset_name(version, target, kind)
    # 使用 replace，存在时强制覆盖
    return setups[0].replace(dest)


def _archive_recipe(target: Target, kind: Kind) -> tuple[Path, tuple[str, ...]]:
    """描述各平台的归档构成"""
    exe = f"rollcaller{target.os.exe_suffix}"

    if kind is Kind.PORTABLE and target.os == OS.WINDOWS:
        return release_dir(target), (exe, *RESOURCES_FILES)
    if kind is Kind.DEVELOP and target.os in (OS.WINDOWS, OS.LINUX):
        return release_dir(target, debug_flag=True), (exe,)

    raise PackageError(f"载荷 {kind!s} 在 {target.os.value!r} 上没有归档配方，"
                       f"请确认 Kind.oses 与 packager 配方表一致")


def _produce_archive(base_dir: Path, resources: Sequence[str], version: Version, target: Target, kind: Kind) -> Path:
    """把相对于 base_dir 的 resources 打包成压缩包"""
    for name in resources:
        if not (base_dir / name).exists():
            raise PackageError(f"{base_dir} 下缺少 {name}，请确认构建产物完整")

    dest = base_dir / asset_name(version, target, kind)
    dest.unlink(missing_ok=True)

    if dest.suffix == ".zip":
        _write_zip(base_dir, resources, dest)
    elif dest.name.endswith(".tar.gz"):
        _write_tar_gz(base_dir, resources, dest)
    else:
        raise PackageError(f"无法根据 {dest.name!r} 的扩展名选择压缩格式，当前仅支持 .zip / .tar.gz")

    return dest


def _members(base_dir: Path, resources: Sequence[str]) -> Iterator[tuple[Path, str]]:
    """磁盘路径 / 包内路径 的稳定序列，目录递归、同级按名字排序"""
    for name in resources:
        yield from _walk(base_dir / name, name)


def _walk(path: Path, name: str) -> Iterator[tuple[Path, str]]:
    yield path, name
    if path.is_dir():
        for child in sorted(path.iterdir(), key=lambda item: item.name):
            yield from _walk(child, f"{name}/{child.name}")


def _write_zip(base_dir: Path, resources: Sequence[str], dest: Path) -> None:
    with zipfile.ZipFile(dest, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for path, name in _members(base_dir, resources):
            archive.write(path, name)


def _write_tar_gz(base_dir: Path, resources: Sequence[str], dest: Path) -> None:
    with tarfile.open(dest, "w:gz") as archive:
        for path, name in _members(base_dir, resources):
            archive.add(path, arcname=name, recursive=False)
