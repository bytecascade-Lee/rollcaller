#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
将生成的产物进行打包
"""

import tarfile
import zipfile
from pathlib import Path
from typing import Iterator, Sequence

from build.builder import release_dir
from common.entity.target import Target
from common.entity.version import Version
from common.enums.kind import Kind


class PackageError(Exception):
    """打包失败"""


def asset_name(version: Version, target: Target, kind: Kind) -> str:
    """产物文件名：rollcaller-<version>-<os>-<arch>-<role>[.<extension>]"""
    return f"rollcaller-{version}-{target.os}-{target.arch}-{kind.role}{kind.extension_with_dot(target.os)}"


def package_setup(version: Version, target: Target, kind: Kind) -> Path:
    """将安装包重命名为最终产物名"""
    setup_dir = release_dir(target, debug_mode=False) / "bundle" / str(kind)
    pattern = f"*-setup{kind.extension_with_dot(target.os)}"
    setups = [p for p in setup_dir.glob(pattern)]
    if len(setups) != 1:
        names = [repr(p.name) for p in setups]
        raise PackageError(f"{str(setup_dir)!r} 下应恰好有一个符合 glob 要求 {pattern!r} 的安装包，"
                           f"但实际有 {len(setups)} 个，分别为 {names!r}")
    dest = setup_dir / asset_name(version, target, kind)
    # 使用 replace，存在时强制覆盖
    return setups[0].replace(dest)


def package_archive(resources: Sequence[str], version: Version, target: Target, kind: Kind,
                    debug_mode: bool = False) -> Path:
    """把相对于构建目录下的 resources 打包成压缩包"""
    base_dir = release_dir(target, debug_mode=debug_mode)
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
