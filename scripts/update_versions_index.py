#!/usr/bin/env python3
"""
更新版本索引 resources/update/versions.json：添加或修改某个版本的 severity/force。

用法:
    uv run scripts/update_versions_index.py <版本号> [--severity normal|important|critical] [--force]

版本号可带 v 也可不带（内部自动去除前导 v，与 versions.json 存储格式一致）。
- 版本已存在：更新其 severity/force
- 版本不存在：新增条目
- 写入后按 (major, minor, patch) 倒序重排（与 publish.py build_versions_asset 的
  versions.json 附件顺序一致，保证源文件与发布产物顺序相同）

约束（与 docs/更新策略与版本索引.md 一致）:
    force=true 仅允许与 severity=critical 组合，否则报错退出，
    防止误把普通版本标成强制更新。
"""

import argparse
import json
from pathlib import Path

from common import version as version_mod
from common.logger import log

ROOT = Path(__file__).resolve().parent.parent

# 版本索引源文件（仓库维护，发布期唯一标定 severity/force 的地方）
VERSIONS_INDEX_PATH = ROOT / "resources" / "update" / "versions.json"

# severity 合法档位（与后端 manifest.rs 的 Severity 枚举、publish.py 一致）
SEVERITY_LEVELS = ("normal", "important", "critical")


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def load_index() -> list:
    """读取并校验 versions.json，返回条目列表（dict: version/severity/force）。"""
    if not VERSIONS_INDEX_PATH.exists():
        fail(f"缺少版本索引源文件: {VERSIONS_INDEX_PATH}")
    try:
        data = json.loads(VERSIONS_INDEX_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        fail(f"versions.json 解析失败: {e}")
    entries = data.get("versions")
    if not isinstance(entries, list):
        fail("versions.json 缺少 versions 数组")
    for item in entries:
        if not isinstance(item, dict) or not item.get("version"):
            fail("versions.json 中存在缺少 version 的条目")
        severity = item.get("severity", "normal")
        if severity not in SEVERITY_LEVELS:
            fail(
                f"版本 {item['version']} 的 severity={severity!r} 非法"
                f"（应为 normal/important/critical）"
            )
    return entries


def write_index(entries: list) -> None:
    """按 (major, minor, patch) 倒序写回，与 publish.py 的 versions.json 附件顺序一致。"""
    entries.sort(
        key=lambda e: version_mod.parse(e["version"])[:3],
        reverse=True,
    )
    VERSIONS_INDEX_PATH.write_text(
        json.dumps({"versions": entries}, ensure_ascii=False, indent=4) + "\n",
        encoding="utf-8",
    )


def main() -> None:
    parser = argparse.ArgumentParser(
        description="更新版本索引 versions.json：添加/修改某版本的 severity/force"
    )
    parser.add_argument(
        "version",
        help="语义化版本号，可带 v 也可不带，例如 v0.7.0 或 0.7.0",
    )
    parser.add_argument(
        "--severity",
        choices=SEVERITY_LEVELS,
        default="normal",
        help=f"严重级别（默认 normal）：{' / '.join(SEVERITY_LEVELS)}",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="强制更新标记；仅允许与 --severity critical 组合",
    )
    args = parser.parse_args()

    # 去除前导 v + 格式校验
    try:
        ver = version_mod.validate(args.version)
    except version_mod.InvalidVersionError as e:
        fail(str(e))

    # force 仅限 critical（防止误把普通版本标成强制更新）
    if args.force and args.severity != "critical":
        fail("--force 仅允许与 --severity critical 组合（防止误把普通版本标成强制更新）")

    entries = load_index()
    entry = next((e for e in entries if e["version"] == ver), None)

    if entry is None:
        entries.append(
            {"version": ver, "severity": args.severity, "force": args.force}
        )
        log("INFO", f"已添加 {ver}: severity={args.severity}, force={args.force}")
    else:
        old_severity, old_force = entry["severity"], entry["force"]
        if old_severity == args.severity and old_force == args.force:
            log(
                "INFO",
                f"{ver} 的 severity/force 未变化"
                f"（severity={args.severity}, force={args.force}），无需修改",
            )
            return
        entry["severity"] = args.severity
        entry["force"] = args.force
        log(
            "INFO",
            f"已更新 {ver}: severity {old_severity}->{args.severity}, "
            f"force {old_force}->{args.force}",
        )

    write_index(entries)
    log("INFO", f"已写入 {VERSIONS_INDEX_PATH.relative_to(ROOT).as_posix()}")


if __name__ == "__main__":
    main()
