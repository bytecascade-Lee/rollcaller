#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
从 app 的路由表（`frontend/src/lib/resources/app/meta.json`）生成 TypeScript 入口声明。

生成物：`frontend/src/lib/routing/entries.generated.ts`，内含

1. `APP_HOST`：本表的 host（= 窗口名），地址 `#/<host>/...` 的第一段；
2. `AppEntryId`：全部入口名的联合类型（表内唯一）；
3. `APP_ENTRIES`：入口 → 归属，带 `as const satisfies` 双重约束——
   值写错（引用了不存在的 parentId）由 `satisfies` 在编译期拦下；
4. `APP_ENTRY_IDS`：入口名的运行时投影（顺序即 meta 的书写顺序）。

设计约定：

- **`meta.json` 是入口名的唯一手写处**。本脚本只读它；生成物不得手改。
- **不生成路径**：地址由 `parentId` 链推出，另有一份常量就是同一事实两份。
- **不生成 camelCase 别名**：`editStudent` / `edit-student` 两个名字谁是权威？
  字面量联合类型已经给出全部好处（补全、写错即报错、全仓可搜）。
- **行尾固定 `\\n`**：`Path.write_text` 在 Windows 文本模式下会把 `\\n` 翻成 `\\r\\n`，
  与 `.editorconfig` 的 `end_of_line = lf` 相悖，故显式 `newline="\\n"`。

用法：

    python scripts/generate_routing_entries.py            # 生成/更新
    python scripts/generate_routing_entries.py --check    # 只校验是否与 meta 一致
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Dict, List, Optional

sys.path.insert(0, str(Path(__file__).resolve().parent))

from common.logger import log

ROOT = Path(__file__).resolve().parents[1]
META_PATH = ROOT / "frontend" / "src" / "lib" / "resources" / "app" / "meta.json"
OUT_PATH = ROOT / "frontend" / "src" / "lib" / "routing" / "entries.generated.ts"

#: 本表的 host（= 窗口名）。表的位置决定 host，不写进节点
HOST = "app"


class MetaError(Exception):
    """路由表本身不合法"""


def read_parent_map() -> Dict[str, Optional[str]]:
    """读取并校验 meta.json，返回 `{入口名: parentId}`（保留书写顺序）"""
    if not META_PATH.exists():
        raise MetaError(f"路由表不存在: {META_PATH}")

    try:
        raw = json.loads(META_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        raise MetaError(f"路由表不是合法 JSON: {e}") from e

    nodes = raw.get("nodes")
    if not isinstance(nodes, dict) or not nodes:
        raise MetaError("路由表缺少非空的 `nodes` 对象")

    parent_of: Dict[str, Optional[str]] = {}
    for name, node in nodes.items():
        if not isinstance(node, dict):
            raise MetaError(f"节点 `{name}` 必须是对象")
        if "parentId" not in node:
            raise MetaError(f"节点 `{name}` 缺少 `parentId`（根节点写 null）")
        parent = node["parentId"]
        if parent is not None and not isinstance(parent, str):
            raise MetaError(f"节点 `{name}` 的 `parentId` 必须是字符串或 null")
        parent_of[name] = parent

    for name, parent in parent_of.items():
        if parent is not None and parent not in parent_of:
            raise MetaError(f"节点 `{name}` 的 parentId `{parent}` 不在表中")
        if parent == name:
            raise MetaError(f"节点 `{name}` 的 parentId 指向自己")

    # 顺着 parentId 往上走，走不到根即为环
    for name in parent_of:
        seen = set()
        cur: Optional[str] = name
        while cur is not None:
            if cur in seen:
                raise MetaError(f"`parentId` 链成环，涉及节点 `{cur}`")
            seen.add(cur)
            cur = parent_of[cur]

    return parent_of


def render(parent_of: Dict[str, Optional[str]]) -> str:
    """渲染生成物内容（行尾 \\n，UTF-8，末尾留一个换行）"""
    names: List[str] = list(parent_of)

    union = "\n".join(f'  | "{name}"' for name in names)

    entries = ",\n".join(
        f'  "{name}": {{"parentId": {json.dumps(parent)}}}' for name, parent in parent_of.items()
    )

    ids = ",\n".join(f'  "{name}"' for name in names)

    return (
        "// 本文件由 scripts/generate_routing_entries.py 生成，请勿手改。\n"
        "// 源：frontend/src/lib/resources/app/meta.json\n"
        "\n"
        "/** 本表的 host（= 窗口名）：地址 `#/<host>/...` 的第一段 */\n"
        f'export const APP_HOST = "{HOST}";\n'
        "\n"
        "/** 全部入口名。表内唯一，也是地址中除 host 外的合法段 */\n"
        "export type AppEntryId =\n"
        f"{union};\n"
        "\n"
        "/** 入口 → 归属。`parentId` 为 null 者是树的根（卡片） */\n"
        "export const APP_ENTRIES = {\n"
        f"{entries}\n"
        "} as const satisfies Record<AppEntryId, {parentId: AppEntryId | null}>;\n"
        "\n"
        "/** 全部入口名（运行时投影，顺序即 meta 的书写顺序） */\n"
        "export const APP_ENTRY_IDS = [\n"
        f"{ids}\n"
        "] as const satisfies readonly AppEntryId[];\n"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="从 app 路由表生成 TS 入口声明")
    parser.add_argument(
        "--check",
        action="store_true",
        help="只校验生成物与路由表是否一致，不写文件（不一致时退出码 1）",
    )
    args = parser.parse_args()

    try:
        parent_of = read_parent_map()
    except MetaError as e:
        log("ERROR", f"路由表校验失败: {e}")
        return 1

    content = render(parent_of)
    root_count = sum(1 for parent in parent_of.values() if parent is None)

    if args.check:
        if not OUT_PATH.exists():
            log("ERROR", f"生成物不存在: {OUT_PATH}")
            return 1
        actual = OUT_PATH.read_text(encoding="utf-8")
        if actual != content:
            log("ERROR", f"生成物与路由表不一致: {OUT_PATH}（重新运行本脚本即可修复）")
            return 1
        log("INFO", f"生成物与路由表一致（{len(parent_of)} 个入口 / {root_count} 个根）")
        return 0

    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with OUT_PATH.open("w", encoding="utf-8", newline="\n") as f:
        f.write(content)

    log("INFO", f"已生成 {OUT_PATH.relative_to(ROOT)}（{len(parent_of)} 个入口 / {root_count} 个根）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
