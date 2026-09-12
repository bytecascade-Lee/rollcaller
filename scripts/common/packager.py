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

from common import targets as target_mod


class PackageError(Exception):
    pass


# Develop 直更 zip 内的文件名——**必须与后端常量 `DEVELOP_UPDATE_BIN_NAME` 逐字符一致**。
DEVELOP_UPDATE_BIN_NAME = "rollcaller-update-from-develop.exe"


def arch_for_target(target: Optional[str]) -> str:
    """根据 --target 返回产物命名用的架构标识；未指定时视为 x86_64。"""
    if not target:
        return "x86_64"
    return target_mod.to_alise(target)


def release_dir(backend: Path, target: Optional[str]) -> Path:
    """根据是否指定 --target 返回对应的 release 目录。"""
    if target:
        return backend / "target" / target / "release"
    return backend / "target" / "release"


def asset_name(version: str, arch: str, kind: str, ext: str) -> str:
    """产物文件名：rollcaller-<version>-windows-<arch>-<kind>.<ext>"""
    return f"rollcaller-{version}-windows-{arch}-{kind}.{ext}"


def package_setup(release_dir_: Path, version: str, arch: str, out_dir: Path) -> Path:
    """将 bundle/nsis 下的安装包重命名为最终产物名并拷贝到 out_dir（**不签名**）。

    只负责"重命名到最终名字"这一步：`.sig` 由调用方在全部产物打包完成后，统一调
    tauri signer sign 生成（见 common/signer.py）。bundler 侧已不再产 `.sig`
    （tauri.conf.json5 的 createUpdaterArtifacts = false），故此处也不得拷贝它，
    否则会与手动签名产物混在同名路径上。

    安装包按 `*-setup.exe` 精确匹配、并要求恰好一个：宁可把异常目录暴露出来，
    也不猜哪个是本次产物。
    """
    nsis_dir = release_dir_ / "bundle" / "nsis"
    setups = [p for p in nsis_dir.glob("*.exe") if p.name.endswith("-setup.exe")]
    if len(setups) != 1:
        names = [p.name for p in setups]
        raise PackageError(
            f"bundle/nsis 下应恰好有一个安装包（*-setup.exe），实际有 {len(setups)} 个: {names}"
        )
    out_dir.mkdir(parents=True, exist_ok=True)
    exe_dst = out_dir / asset_name(version, arch, "setup", "exe")
    shutil.copy2(setups[0], exe_dst)
    return exe_dst


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


def package_develop(debug_dir: Path, version: str, arch: str, out_dir: Path) -> Path:
    """将 debug 构建的 rollcaller.exe 打包为 Develop 直更 zip。

    用于开发模式（AppMode::Develop）的自更新演练：客户端下载该 zip 校验后解压出目录
    （内含 `DEVELOP_UPDATE_BIN_NAME` 指定的 debug exe），由 Go updater 以"不清空 target"的 config 写入
    `backend/target/debug/` 同名文件。**zip 内文件名必须是该常量**（而非 rollcaller.exe）：
    既与 cargo 产物/IDE 映射的文件区分开（绕开镜像占用），也与后端启动路径同名。
    签名由调用方执行（signer.sign_artifact）。
    """
    exe = debug_dir / "rollcaller.exe"
    if not exe.is_file():
        raise PackageError(f"debug 目录下缺少 rollcaller.exe: {debug_dir}")
    out_dir.mkdir(parents=True, exist_ok=True)
    dest = out_dir / asset_name(version, arch, "develop", "zip")
    with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as zf:
        zf.write(exe, DEVELOP_UPDATE_BIN_NAME)
    return dest
