#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
从两个域的声明生成 TypeScript 导航入口。

- **app**：`frontend/src/lib/resources/app/meta.json`（节点树）
- **help**：`frontend/src/lib/resources/help/meta.json`（文档树）+ 各文档 md（锚点）

生成物 `frontend/src/lib/routing/entries.generated.ts` 内含

app 段：

1. `APP_HOST`：host
2. `AppEntryId`：入口名
3. `AppEntry`：入口句柄
4. `APP_ENTRY_PARENT`：入口 → 父入口

help 段：

5. `HelpEntry`：深链句柄 → 地址

## help 的句柄怎么来

没有第二份声明，全部由 meta + md 推出：

- **文档级**：`help/meta.json` 里**能找到 md 源**的节点各一个句柄（`quick-start` → `quickStart`）
  → `#/help/quick-start`。md 源有两处（由 `doc_sources` 定位）：
  - `resources/help/<id>/` 目录 —— 正文型文档；
  - **仓库根同名文件** —— `README.md`/`LICENSE`/`CHANGELOG.md`/`RELEASE_NOTES.md`，
    它们的正文由独立命令加载，但文件与锚点都在，故一并生成。
  只有既无目录又无同名文件的分组节点（`rollcall`/`students`/`records`/`ui`/`about`/`multi-rollcall`）
  没有句柄。
- **锚点级**：文档 md 里每个 `{#anchor}` 声明各一个句柄，名为「文档句柄 + 锚点句柄」
  （`tts` + `modes` → `ttsModes`）→ `#/help/tts/modes`。
  同名锚点（`app-window` 与 `help-window` 都有 `titlebar`）因此天然区分。
  同一目录下有多份语言版本时取**并集**（地址不带语言段，锚点必须在任意语言下都可达）。

用法：

    python scripts/generate_routing_entries.py            # 生成/更新
    python scripts/generate_routing_entries.py --check    # 只校验是否与声明一致
"""

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple

sys.path.insert(0, str(Path(__file__).resolve().parent))

from common.logger import log

ROOT = Path(__file__).resolve().parents[1]

APP_META_PATH = ROOT / "frontend" / "src" / "lib" / "resources" / "app" / "meta.json"
HELP_META_PATH = ROOT / "frontend" / "src" / "lib" / "resources" / "help" / "meta.json"
HELP_DOC_ROOT = ROOT / "resources" / "help"
OUT_PATH = ROOT / "frontend" / "src" / "lib" / "routing" / "entries.generated.ts"

#: 两个域的 host（= 窗口名）。表的位置决定 host，不写进节点
APP_HOST = "app"
HELP_HOST = "help"

#: 应用内地址前缀，与 `frontend/src/lib/routing/address.ts` 的 `ROUTE_PREFIX` 一致
ROUTE_PREFIX = "#/"

_IDENT_RE = re.compile(r"^[A-Za-z_$][A-Za-z0-9_$]*$")
#: 标题锚点声明 `## 标题 {#kebab-name}`
_ANCHOR_RE = re.compile(r"\{#([A-Za-z0-9_-]+)\}")
#: 锚点语法：小写字母数字，单连字符分隔（与 `helpAddress.ts` 的 `KEBAB_RE` 一致）
_KEBAB_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def handle_of(name: str) -> str:
    """名字 → 代码句柄：`edit-student` → `editStudent`，`RELEASE_NOTES.md` → `releaseNotes`

    先去掉扩展名，再按 `-`/`_` 断词；**逐段小写归一**后首段保持小写、其余段首字母大写
    （`RELEASE_NOTES` 必须给出 `releaseNotes` 而不是 `releaseNOTES`）。
    """
    stem = re.sub(r"\.[A-Za-z0-9]+$", "", name)
    head, *rest = re.split(r"[-_]+", stem)
    return head.lower() + "".join(part[:1].upper() + part[1:].lower() for part in rest)


def join_handle(prefix: str, suffix: str) -> str:
    """两级句柄拼一级：`quick-start` + `install-app` → `quickStartInstallApp`"""
    return prefix + suffix[:1].upper() + suffix[1:]


class MetaError(Exception):
    """声明本身不合法"""


def read_parent_map(meta_path: Path) -> Dict[str, Optional[str]]:
    """读取并校验一张节点表，返回 `{节点名: parentId}`（保留书写顺序）"""
    if not meta_path.exists():
        raise MetaError(f"节点表不存在: {meta_path}")

    try:
        raw = json.loads(meta_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        raise MetaError(f"节点表不是合法 JSON: {e}") from e

    nodes = raw.get("nodes")
    if not isinstance(nodes, dict) or not nodes:
        raise MetaError(f"{meta_path.name} 缺少非空的 `nodes` 对象")

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


def check_handles(pairs: List[Tuple[str, str, str]]) -> None:
    """句柄必须合法且互不相同（否则生成出来的 TS 会静默互相覆盖）

    :param pairs: `(域标签, 句柄, 来源说明)` 列表
    """
    owner: Dict[str, str] = {}
    for label, handle, source in pairs:
        if not _IDENT_RE.match(handle):
            raise MetaError(f"[{label}] {source} 推不出合法的代码句柄（得到 `{handle}`）")
        if handle in owner:
            raise MetaError(
                f"[{label}] {source} 与 {owner[handle]} 推出同一个句柄 `{handle}`，"
                "请改名使其在去掉连字符后仍可区分"
            )
        owner[handle] = source


def doc_sources(doc: str) -> List[Path]:
    """一篇文档的 md 源。两处各查一次：`resources/help/<id>/` 目录，或仓库根同名文件

    （`README.md`/`LICENSE`/`CHANGELOG.md`/`RELEASE_NOTES.md` 的正文不在 resources 下，
    文件却在仓库根，锚点也照常声明 —— 故一并当作文档源。）
    """
    doc_dir = HELP_DOC_ROOT / doc
    if doc_dir.is_dir():
        return sorted(doc_dir.glob("*.md"))

    root_file = ROOT / doc
    return [root_file] if root_file.is_file() else []


def scan_anchors(source: str, mds: List[Path]) -> List[str]:
    """扫描一篇文档所有 md 源里的锚点声明，按首次出现的顺序取并集"""
    anchors: List[str] = []
    for md in mds:
        rel = md.relative_to(ROOT).as_posix()
        for anchor in _ANCHOR_RE.findall(md.read_text(encoding="utf-8")):
            if not _KEBAB_RE.match(anchor):
                raise MetaError(f"{rel} 里的锚点 `{anchor}` 不是 kebab-case，无法进地址")
            if anchor not in anchors:
                anchors.append(anchor)
    return anchors


def render_app(parent_of: Dict[str, Optional[str]]) -> str:
    """渲染 app 段"""
    names: List[str] = list(parent_of)

    union = "\n".join(f'  | "{name}"' for name in names)
    handles = ",\n".join(f'  {handle_of(name)}: "{name}"' for name in names)
    parents = ",\n".join(
        f'  "{name}": {json.dumps(parent)}' for name, parent in parent_of.items()
    )

    return (
        "/** host */\n"
        f'export const APP_HOST = "{APP_HOST}";\n'
        "\n"
        "/** 入口名 */\n"
        "export type AppEntryId =\n"
        f"{union};\n"
        "\n"
        "/**\n"
        " * 入口句柄\n"
        " */\n"
        "export const AppEntry = {\n"
        f"{handles}\n"
        "} as const satisfies Record<string, AppEntryId>;\n"
        "\n"
        "/** 入口 → 父入口；null 表示卡片 */\n"
        "export const APP_ENTRY_PARENT = {\n"
        f"{parents}\n"
        "} as const satisfies Record<AppEntryId, AppEntryId | null>;\n"
    )


def render_help(parent_of: Dict[str, Optional[str]], anchors: Dict[str, List[str]]) -> str:
    """渲染 help 段"""
    lines: List[str] = []
    skipped: List[str] = []

    for doc in parent_of:
        if doc not in anchors:
            skipped.append(doc)
            continue
        lines.append(f'  {handle_of(doc)}: "{ROUTE_PREFIX}{HELP_HOST}/{doc}"')
        for anchor in anchors[doc]:
            handle = join_handle(handle_of(doc), handle_of(anchor))
            lines.append(f'  {handle}: "{ROUTE_PREFIX}{HELP_HOST}/{doc}/{anchor}"')

    if not lines:
        raise MetaError("没有一篇文档能找到 md 源，HelpEntry 会生成空表")

    note = ""
    if skipped:
        note = (
            "// 未生成句柄的节点（既无 resources/help/<id>/ 目录、也无仓库根同名文件）："
            + ", ".join(skipped)
            + "\n"
        )

    return (
        "/**\n"
        " * 帮助窗口的深链句柄：句柄 → 地址（`#/help/<文档>[/<锚点>]`）\n"
        " *\n"
        " * 由 `help/meta.json` 的文档树与各文档 md 里的 `{#锚点}` 派生，\n"
        " * 文档级句柄为文档名、锚点级句柄为文档名 + 锚点名。\n"
        " * md 源 = `resources/help/<id>/` 目录，或仓库根同名文件（README.md / LICENSE / …）。\n"
        " */\n"
        f"{note}"
        "export const HelpEntry = {\n"
        + ",\n".join(lines)
        + "\n} as const satisfies Record<string, `#/help/${string}`>;\n"
    )


def render(
    app_parent_of: Dict[str, Optional[str]],
    help_parent_of: Dict[str, Optional[str]],
    help_anchors: Dict[str, List[str]],
) -> str:
    """渲染生成物内容（行尾 \\n，UTF-8，末尾留一个换行）"""
    return (
        "// This file is generated by scripts/generate_routing_entries.py, please do not manually modify it.\n"
        "// source: frontend/src/lib/resources/app/meta.json\n"
        "// source: frontend/src/lib/resources/help/meta.json + *.md\n"
        "\n"
        + render_app(app_parent_of)
        + "\n"
        + render_help(help_parent_of, help_anchors)
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="从 app / help 的声明生成 TS 导航入口")
    parser.add_argument(
        "--check",
        action="store_true",
        help="只校验生成物与声明是否一致，不写文件（不一致时退出码 1）",
    )
    args = parser.parse_args()

    try:
        app_parent_of = read_parent_map(APP_META_PATH)
        # 句柄唯一性必须在生成前查：同一域内重名会静默覆盖成一条
        check_handles(
            [("app", handle_of(name), f"节点 `{name}`") for name in app_parent_of]
        )

        help_parent_of = read_parent_map(HELP_META_PATH)
        help_anchors: Dict[str, List[str]] = {}
        help_pairs = []
        for doc in help_parent_of:
            mds = doc_sources(doc)
            if not mds:
                continue
            anchors = scan_anchors(doc, mds)
            help_anchors[doc] = anchors
            help_pairs.append(("help", handle_of(doc), f"文档 `{doc}`"))
            for anchor in anchors:
                help_pairs.append(
                    (
                        "help",
                        join_handle(handle_of(doc), handle_of(anchor)),
                        f"`{doc}` 的锚点 `{anchor}`",
                    )
                )
        check_handles(help_pairs)
    except MetaError as e:
        log("ERROR", f"声明校验失败: {e}")
        return 1

    content = render(app_parent_of, help_parent_of, help_anchors)
    app_roots = sum(1 for parent in app_parent_of.values() if parent is None)
    anchor_count = sum(len(anchors) for anchors in help_anchors.values())
    summary = (
        f"{len(app_parent_of)} 个入口 / {app_roots} 个根 · "
        f"{len(help_anchors)} 篇文档 / {anchor_count} 个锚点"
    )

    if args.check:
        if not OUT_PATH.exists():
            log("ERROR", f"生成物不存在: {OUT_PATH}")
            return 1
        actual = OUT_PATH.read_text(encoding="utf-8")
        if actual != content:
            log("ERROR", f"生成物与声明不一致: {OUT_PATH}（重新运行本脚本即可修复）")
            return 1
        log("INFO", f"生成物与声明一致（{summary}）")
        return 0

    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with OUT_PATH.open("w", encoding="utf-8", newline="\n") as f:
        f.write(content)

    log("INFO", f"已生成 {OUT_PATH.relative_to(ROOT)}（{summary}）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
