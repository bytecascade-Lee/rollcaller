#!/usr/bin/env python3
"""
versions.json（版本索引）与本地产物目录的处理模块。

# 两种数据源

- 仓库源 `resources/update/versions.json`：发布期**唯一标定 severity** 的地方，
  publish_ci.py 读取后随 Release 发布 versions.json 附件（客户端据此做坏版本/
  历史严重级别检测）；
- 本地产物目录 `release/Local/`：publish_local.py 扫描全部已构建版本目录生成
  serve_local.py 所需的 versions.json（severity 沿仓库源标定，未标定按 normal）。

此外本地产物目录的版本选择逻辑（`<core>+<branch>.<count>.<hash>` 解析、按
core 前缀选目录、同版本多目录取提交数最多者）serve_local.py 与 publish_local.py
共用，也收口在本模块，避免三处各自实现后再次漂移。

severity 合法档位与后端 Severity 枚举一致（复用 common.manifest.SEVERITY_LEVELS）。
"""

import json
from pathlib import Path
from typing import Dict, List

from common import version as version_mod
from common.manifest import SEVERITY_LEVELS

# release/Local 的默认相对路径（相对项目根）；与 .gitignore 的 /release/ 一致
DEFAULT_LOCAL_DIR = ("release", "Local")


class VersionIndexError(ValueError):
    """versions.json / 目录选择相关错误（沿用 ValueError 语义，由调用方终止）。"""


def fail(message: str) -> None:
    raise VersionIndexError(message)


def default_local_dir(root: Path) -> Path:
    """项目根 → 默认本地产物根目录 release/Local。"""
    return root.joinpath(*DEFAULT_LOCAL_DIR)


def build_count_from_dirname(dir_name: str) -> int:
    """从 `<core>+<branch>.<count>.<hash>` 中解析提交数（build 段倒数第 2 段）。

    分支名可含 '.'，因此不能按固定下标取；build 段末尾恒为 `<count>.<short_hash>`。
    解析失败返回 -1（不参与"取提交数最多"的择优）。
    """
    if "+" not in dir_name:
        return -1
    parts = dir_name.split("+", 1)[1].split(".")
    try:
        return int(parts[-2])
    except (IndexError, ValueError):
        return -1


def scan_version_dirs(local_dir: Path) -> Dict[str, List[Path]]:
    """扫描本地产物根目录 → {core 版本号: [目录, ...]}。

    目录名 `<core>+<branch>.<count>.<hash>`；core 为第一个 '+' 之前且可通过
    版本号校验的部分。core 非法的目录忽略（不报错——目录可能并非产物目录）。
    """
    found: Dict[str, List[Path]] = {}
    if not local_dir.is_dir():
        return found
    for d in local_dir.iterdir():
        if not d.is_dir() or "+" not in d.name:
            continue
        core = d.name.split("+", 1)[0]
        try:
            version_mod.validate(core)
        except version_mod.VersionError:
            continue
        found.setdefault(core, []).append(d)
    return found


def pick_version_dir(local_dir: Path, version: str) -> Path:
    """在本地产物根目录中挑选 `version+` 前缀的产物目录。

    - 无匹配目录 → ValueError（提示先执行本地构建）；
    - 同版本多个目录 → 取提交数（build 段倒数第 2 段）最多者，并列取修改时间新者。

    Args:
        local_dir: 本地产物根目录（如 release/Local）
        version: 核心版本号（不含 v、不含 +build）
    """
    version = version_mod.validate(version)
    if not local_dir.is_dir():
        fail(f"产物根目录不存在: {local_dir}")
    candidates = [
        d for d in local_dir.iterdir()
        if d.is_dir() and d.name.startswith(f"{version}+")
    ]
    if not candidates:
        fail(
            f"{local_dir} 下未找到版本 {version} 的构建产物目录 "
            f"（期望形如 {version}+<分支>.<提交数>.<短哈希>）。\n"
            f"请先执行本地构建: uv run python scripts/release_local.py {version}"
        )
    return max(
        candidates,
        key=lambda d: (build_count_from_dirname(d.name), d.stat().st_mtime),
    )


def read_entries(path: Path) -> Dict[str, str]:
    """读取裸数组 versions.json → {版本号: severity}（版本号已规范化，去前导 v）。

    severity 缺失/非法按 normal 兜底（老索引兼容；与后端 HistoryVersion 缺省语义一致）。
    文件缺失或不可解析抛 ValueError。
    """
    if not path.is_file():
        fail(f"缺少版本索引文件: {path}")
    try:
        entries = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        fail(f"{path} 解析失败: {e}")
    if not isinstance(entries, list):
        fail(f"{path} 不是合法的裸数组版本索引")
    index = {}
    for item in entries:
        if not isinstance(item, dict):
            continue
        raw = item.get("version")
        if not raw:
            fail(f"{path} 中存在缺少 version 的条目")
        index[version_mod.validate(raw)] = item.get("severity", "normal")
    return index


def write_entries(path: Path, entries: List[dict]) -> Path:
    """写入裸数组 versions.json（统一缩进与结尾换行）。"""
    path.write_text(
        json.dumps(entries, ensure_ascii=False, indent=4) + "\n",
        encoding="utf-8",
    )
    return path


def build_entries(severity_map: Dict[str, str]) -> List[dict]:
    """{版本号: severity} → 按语义化版本倒序的裸数组条目 [{version, severity}, ...]。

    排序键取主/次/补丁三段数值（预发布版本与同名正式版并列时保持输入稳定序，
    与历史 publish/scan 行为一致）。
    """
    return [
        {"version": ver, "severity": sev}
        for ver, sev in sorted(
            severity_map.items(),
            key=lambda kv: version_mod.parse(kv[0])[:3],
            reverse=True,
        )
    ]
