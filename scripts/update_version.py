#!/usr/bin/env python3
"""统一同步四个文件中的版本号：
    pyproject.toml / backend/tauri.conf.json5 / backend/Cargo.toml / frontend/package.json

用法:
    python scripts/update_version.py <版本号>

版本号可带 v 也可不带（内部自动去掉前导 v），支持 alpha/beta 等预发布标识。
采用行匹配替换，无需任何第三方依赖。

修改完成后会打印各文件 旧版本 -> 新版本 的同步结果。
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path
from typing import List

from common import version as version_mod
from common.logger import log

ROOT = Path(__file__).resolve().parent.parent

# (文件路径, 行匹配正则, 替换后行内容模板)
# 各文件只替换第一处匹配（均为顶层版本号字段，依赖版本不受影响）
FILES = [
    (
        ROOT / "pyproject.toml",
        re.compile(r'^(version\s*=\s*")([^"]*)(")', re.MULTILINE),
    ),
    (
        ROOT / "backend" / "tauri.conf.json5",
        re.compile(r'^(\s*version\s*:\s*")([^"]*)(")', re.MULTILINE),
    ),
    (
        ROOT / "backend" / "Cargo.toml",
        re.compile(r'^(version\s*=\s*")([^"]*)(")', re.MULTILINE),
    ),
    (
        ROOT / "frontend" / "package.json",
        re.compile(r'^(\s*"version"\s*:\s*")([^"]*)(")', re.MULTILINE),
    ),
]

LOCKS = [
    (
        ROOT,
        ["uv", "lock"]
    )
]


def normalize_version(raw: str) -> str:
    """去除可选的 v 前缀并校验语义化版本格式（复用 common.version）。"""
    try:
        return version_mod.validate(raw, min_level=None)
    except version_mod.VersionError as e:
        log("ERROR", f"非法版本号: {raw!r}：{e}")
        raise SystemExit(1)


def update_file(path: Path, pattern: re.Pattern, version: str) -> str:
    """行匹配替换文件中的版本号，返回旧版本号。"""
    if not path.exists():
        log("ERROR", f"文件不存在: {path}")
        raise SystemExit(1)
    content = path.read_text(encoding="utf-8")
    match = pattern.search(content)
    if not match:
        log("ERROR", f"未在 {path} 中找到版本号字段，格式可能已变化")
        raise SystemExit(1)
    old_version = match.group(2)
    new_content, count = pattern.subn(
        lambda m: m.group(1) + version + m.group(3), content, count=1
    )
    if new_content == content:
        log("WARNING", f"版本号相同: {path}")
        return old_version
    if count != 1:
        log("ERROR", f"替换失败: {path}")
        raise SystemExit(1)
    path.write_text(new_content, encoding="utf-8")
    return old_version


def sync_lockfile(workspace: Path, cmd: List[str]) -> bool:
    """在指定 workspace 目录执行锁文件同步命令，成功返回 True。"""
    try:
        result = subprocess.run(
            cmd,
            cwd=workspace,
            capture_output=True,
            text=True,
            encoding='utf-8',
            errors='replace',
            check=False,
        )
        if result.returncode != 0:
            log("WARNING", f"锁文件同步失败 ({' '.join(cmd)}): {result.stderr.strip()}")
            return False
        return True
    except Exception as e:
        log("WARNING", f"锁文件同步异常: {e}")
        return False


def main() -> None:
    parser = argparse.ArgumentParser(
        description="统一同步 pyproject.toml / tauri.conf.json5 / Cargo.toml / package.json 的版本号"
    )
    parser.add_argument(
        "version",
        help="语义化版本号，可带 v 也可不带，例如 v0.1.0-beta.2 或 0.1.0-rc.1",
    )
    args = parser.parse_args()

    version = normalize_version(args.version)

    log("INFO", f"开始同步版本号: {version}")
    for path, pattern in FILES:
        old = update_file(path, pattern, version)
        log("INFO", f"同步文件: {path.relative_to(ROOT).as_posix()}: {old} -> {version}")
    log("INFO", f"四个文件已同步为版本号: {version}")
    log("INFO", "开始同步锁文件")
    for workspace, cmd in LOCKS:
        sync_lockfile(workspace, cmd)
        log("INFO", f"同步命令: {' '.join(cmd)}")
    log("INFO", "一个锁文件已同步")


if __name__ == "__main__":
    main()
