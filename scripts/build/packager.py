#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
将生成的产物进行打包
"""

from pathlib import Path
from typing import List

from build.builder import release_dir
from common.entity.target import Target
from common.entity.version import Version
from common.enums.kind import Kind
from util.un_zip import compress


class PackageError(Exception):
    pass


# 已废弃
DEVELOP_UPDATE_BIN_NAME = "rollcaller-update-from-develop.exe"


def asset_name(version: Version, target: Target, kind: Kind) -> str:
    """产物文件名：rollcaller-<version>-<os>-<arch>-<role>[.<extension>]"""
    return f"rollcaller-{version}-{target.os}-{target.arch}-{kind.role}{kind.extension_with_dot(target.os)}"


def package_setup(version: Version, target: Target, kind: Kind) -> Path:
    """将安装包重命名为最终产物名"""
    setup_dir = release_dir(target, debug_mode=False) / "bundle" / str(kind)
    setups = [p for p in setup_dir.glob(f"*-setup.{kind.extension_with_dot(target.os)}")]
    if len(setups) != 1:
        names = [p.name for p in setups]
        raise PackageError(f"{setup_dir} 下应恰好有一个符合要求的安装包，实际有 {len(setups)} 个: {names}")
    dest = setup_dir / asset_name(version, target, kind)
    # 存在时强制覆盖
    setups[0].replace(dest)
    return dest


def package_zip(resources: List[str], version: Version, target: Target, kind: Kind, debug_mode: bool = False) -> Path:
    """将产物以及资源压缩为压缩包"""
    zip_dir = release_dir(target, debug_mode=debug_mode)
    for name in resources:
        if not (zip_dir / name).exists():
            raise PackageError(f"{zip_dir} 下缺少 {name}，请确认构建产物完整")
    dest = zip_dir / asset_name(version, target, kind)
    return compress(zip_dir, resources, dest, overwrite=True)
