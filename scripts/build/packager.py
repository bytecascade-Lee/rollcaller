#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
将生成的产物进行打包
"""

import shutil
from pathlib import Path
from typing import List

from common.entity.target import Target
from common.entity.version import Version
from common.enums.kind import Kind
from util.un_zip import compress


class PackageError(Exception):
    pass


# 已废弃
DEVELOP_UPDATE_BIN_NAME = "rollcaller-update-from-develop.exe"


def asset_name(version: Version, target: Target, kind: Kind) -> str:
    """产物文件名：rollcaller-<version>-<os>-<arch>-<role>.<extension>"""
    return f"rollcaller-{version}-{target.os}-{target.arch}-{kind.role}.{kind.extension}"


def package_setup(release_dir: Path, version: Version, target: Target, kind: Kind, output_dir: Path) -> Path:
    """将安装包重命名为最终产物名并拷贝到 output_dir"""
    setups = [p for p in release_dir.glob(f"{version}*.{kind.extension}")]
    if len(setups) != 1:
        names = [p.name for p in setups]
        raise PackageError(f"{release_dir} 下应恰好有一个符合要求的安装包，实际有 {len(setups)} 个: {names}")
    output_dir.mkdir(parents=True, exist_ok=True)
    dest = output_dir / asset_name(version, target, kind)
    shutil.copy2(setups[0], dest)
    return dest


def package_zip(release_dir: Path, resources: List[str], version: Version,
                target: Target, kind: Kind, output_dir: Path) -> Path:
    """将产物以及资源压缩为压缩包"""
    for name in resources:
        if not (release_dir / name).exists():
            raise PackageError(f"{release_dir} 下缺少 {name}，请确认构建产物完整")
    output_dir.mkdir(parents=True, exist_ok=True)
    dest = output_dir / asset_name(version, target, kind)
    return compress(release_dir, resources, dest, overwrite=True)
