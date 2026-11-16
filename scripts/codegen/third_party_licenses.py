#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
第三方许可证清单

调用时机：
    开发、构建前
"""

import json
import re
from collections import Counter
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from cli import cargo, pnpm
from common import support
from common.constant.paths import PROJECT_BASE_DIR, THIRD_PARTY_NOTICES_PATH
from common.enums.action import Action
from common.logger import info, success


class NoticesError(Exception):
    """依赖的许可证无法判定：宁可失败，也不能在清单里写出一个错的许可"""


@dataclass(frozen=True, slots=True)
class Component:
    """一个随安装包分发的第三方组件"""

    name: str
    version: str
    licenses: tuple[str, ...]
    copyright: str
    root: Path | None


#: `Apache-2.0 WITH LLVM-exception` 在 SPDX 里是一个整体
LLVM_EXCEPTION = "Apache-2.0 WITH LLVM-exception"

#: OR 表达式的择一顺序：越靠前越优先
LICENSE_PREFERENCE: tuple[str, ...] = (
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
    "BSD-2-Clause",
    "ISC",
    "Zlib",
    "MIT-0",
    "0BSD",
    "CC0-1.0",
    "Unlicense",
    "Unicode-3.0",
    "CDLA-Permissive-2.0",
    "BSL-1.0",
    "PSF-2.0",
    "BlueOak-1.0.0",
    "MPL-2.0",
    "bzip2-1.0.6",
)

#: 上游没在包内声明许可、但同一项目的 crate 已声明 MIT 的组件
LICENSE_OVERRIDES: dict[str, str] = {
    # `@fltsci/tauri-plugin-tracing` 的 npm 包既无 license 字段也无 LICENSE 文件，且是 restricted 发布；
    # 同版本 crate `tauri-plugin-tracing 0.3.4` 声明 MIT，README 徽章亦为 MIT
    "@fltsci/tauri-plugin-tracing": "MIT",
}

#: 视同"没有声明"的占位值
_DECLARED_NOTHING = frozenset({"", "unknown", "unlicensed", "none", "see license"})

#: SPDX 表达式里的关键字，不是许可 id
_KEYWORDS = frozenset({"AND", "OR", "WITH"})

_IDENT_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9.+-]*")
_LICENSE_FILE_RE = re.compile(r"(?i)^(licen[cs]e|copying)")
# 真声明一律以大写 Copyright 或 © 起头；许可正文里被折行切出来的
# "copyright notice that is included..." / "copyright license to reproduce..." 是小写开头
_COPYRIGHT_LINE_RE = re.compile(r"^\s*(?:Copyright\b|©|\(C\))")
_COPYRIGHT_NOISE_RE = re.compile(r"(?i)copyright holders|notice that is included|license to reproduce")
_COPYRIGHT_PLACEHOLDER_RE = re.compile(r"(?i)\[yyyy]|\{yyyy}|\[name of copyright owner]|<year>|\byyyy\b")
_BLANK_RUN_RE = re.compile(r"\n{3,}")

#: 单个许可文件的上限：超过它的一律当成"不是许可正文"（有的包会把整本书塞进 LICENSE）
_MAX_LICENSE_BYTES = 48_000

#: 许可正文的特征串，顺序即优先级
#: BSD-3 必须先于 BSD-2、MIT-0 必须先于 MIT、bzip2 必须先于 Zlib
_SIGNATURES: tuple[tuple[tuple[str, ...], tuple[str, ...]], ...] = (
    # bzip2 的正文是 BSD 句式 + "libbzip2" 冠名，必须先于 BSD-2 命中
    (("bzip2-1.0.6",), ("libbzip2",)),
    ((LLVM_EXCEPTION, "Apache-2.0"), ("llvm exception",)),
    (("Apache-2.0",), ("apache license", "version 2.0")),
    (("MPL-2.0",), ("mozilla public license version 2.0",)),
    (("Unicode-3.0",), ("unicode license v3",)),
    (("Unicode-3.0",), ("unicode, inc. license agreement",)),
    (("CDLA-Permissive-2.0",), ("community data license agreement",)),
    (("BlueOak-1.0.0",), ("blue oak model license",)),
    (("BSL-1.0",), ("boost software license",)),
    (("PSF-2.0",), ("python software foundation license",)),
    (("CC0-1.0",), ("cc0 1.0 universal",)),
    (("Unlicense",), ("free and unencumbered software released into the public domain",)),
    (("MIT-0",), ("mit no attribution",)),
    (("BSD-3-Clause",), ("redistribution and use in source and binary forms", "neither the name")),
    (("BSD-2-Clause",), ("redistribution and use in source and binary forms",)),
    (("ISC",), ("permission to use, copy, modify, and/or distribute",)),
    (("MIT",), ("permission is hereby granted, free of charge",)),
    (("Zlib",), ("without any express or implied warranty",)),
)


def _read_text(path: Path) -> str | None:
    """读成 UTF-8 文本；二进制或读不出来时返回 None"""
    try:
        raw = path.read_bytes()
    except OSError:
        return None
    if b"\x00" in raw[:4096]:
        return None
    return raw.decode("utf-8", errors="replace")


def _unwrap(expression: str) -> str:
    """剥掉整体包裹的括号：`(MPL-2.0 OR Apache-2.0)` → `MPL-2.0 OR Apache-2.0`"""
    while expression.startswith("(") and expression.endswith(")"):
        depth = 0
        for index, char in enumerate(expression):
            depth += 1 if char == "(" else (-1 if char == ")" else 0)
            if depth == 0 and index != len(expression) - 1:
                return expression
        expression = expression[1:-1].strip()
    return expression


def _split_top(expression: str, separator: str) -> list[str]:
    """按不在括号内的 `separator` 切分"""
    parts: list[str] = []
    depth = 0
    start = 0
    index = 0
    while index < len(expression):
        char = expression[index]
        if char == "(":
            depth += 1
        elif char == ")":
            depth = max(depth - 1, 0)
        elif depth == 0 and expression.startswith(separator, index):
            parts.append(expression[start:index])
            index += len(separator)
            start = index
            continue
        index += 1
    parts.append(expression[start:])
    return [part.strip() for part in parts if part.strip()]


def _all_ids(expression: str) -> tuple[str, ...]:
    """把表达式里出现的许可 id 全取出来（AND 组合与 WITH 组合只能整体遵守）"""
    return tuple(dict.fromkeys(token for token in _IDENT_RE.findall(expression) if token not in _KEYWORDS))


def license_ids(declaration: str) -> tuple[str, ...]:
    """
    许可声明 → 需要遵守的许可 id

    表达式里的 `/` 是历史上把 OR 写歪的产物（`MIT/Apache-2.0`、`Unlicense/MIT`），先归一成 OR；
    顶层是 OR 时按 `LICENSE_PREFERENCE` 择一，避免把 LGPL 这类没人会选的选项带进清单；
    顶层是 AND（或 `WITH`）时无法择一，全部保留。
    """
    expression = _unwrap(" ".join(declaration.replace("/", " OR ").split()))
    if not expression or expression.lower() in _DECLARED_NOTHING:
        return ()
    if " AND " in expression or " WITH " in expression:
        return _all_ids(expression)

    alternatives = [_unwrap(alternative) for alternative in _split_top(expression, " OR ")]
    for wanted in LICENSE_PREFERENCE:
        if wanted in alternatives:
            return (wanted,)
    for alternative in alternatives:
        if _IDENT_RE.fullmatch(alternative):
            return (alternative,)
    return _all_ids(expression)


def _detect_ids(name: str, text: str) -> tuple[str, ...]:
    """按正文特征认出许可 id（一个文件可能同时是 `Apache-2.0` 与带 LLVM 例外的版本）"""
    lowered = text.lower()
    for ids, markers in _SIGNATURES:
        if all(marker in lowered for marker in markers):
            return ids
    raised = name.upper()
    if "MIT" in raised:
        return ("MIT",)
    if "APACHE" in raised:
        return ("Apache-2.0",)
    if "BSD" in raised:
        return ("BSD-3-Clause",) if "3" in raised else ("BSD-2-Clause",)
    return ()


def _clean_license_text(text: str) -> str:
    """去掉正文之前属于某个组件的版权抬头，再压掉多余空行"""
    kept: list[str] = []
    in_header = True
    for line in text.replace("\r\n", "\n").replace("\r", "\n").split("\n"):
        stripped = line.rstrip()
        if in_header:
            if not stripped.strip():
                continue
            if _COPYRIGHT_LINE_RE.match(stripped):
                continue
            in_header = False
        kept.append(stripped)
    return _BLANK_RUN_RE.sub("\n\n", "\n".join(kept)).strip()


def _scan_licenses(root: Path | None) -> dict[str, str]:
    """扫一个包目录，返回 `{许可 id: 正文}`"""
    found: dict[str, str] = {}
    if root is None or not root.is_dir():
        return found
    for entry in sorted(root.iterdir()):
        if not entry.is_file() or not _LICENSE_FILE_RE.match(entry.name):
            continue
        if entry.stat().st_size > _MAX_LICENSE_BYTES:
            continue
        text = _read_text(entry)
        if text is None:
            continue
        cleaned = _clean_license_text(text)
        for license_id in _detect_ids(entry.name, text):
            found.setdefault(license_id, cleaned)
    return found


def _copyright_of(root: Path | None, fallback: str) -> str:
    """从包内许可文件里抽版权行；抽不到就退回作者字段"""
    declarations: list[str] = []
    if root is not None and root.is_dir():
        for entry in sorted(root.iterdir()):
            if not entry.is_file() or not _LICENSE_FILE_RE.match(entry.name):
                continue
            text = _read_text(entry)
            if text is None:
                continue
            for line in text.split("\n"):
                candidate = " ".join(line.split())
                if not _COPYRIGHT_LINE_RE.match(candidate):
                    continue
                if _COPYRIGHT_PLACEHOLDER_RE.search(candidate) or _COPYRIGHT_NOISE_RE.search(candidate):
                    continue
                if candidate not in declarations:
                    declarations.append(candidate)
    if declarations:
        return "; ".join(declarations[:2])
    return fallback


def _is_local(manifest_path: str) -> bool:
    """工作区自身的 crate 不算第三方"""
    try:
        return Path(manifest_path).is_relative_to(PROJECT_BASE_DIR)
    except OSError, ValueError:
        return False


def _undeclared_error(name: str) -> NoticesError:
    return NoticesError(
        f"组件 {name!r} 没有可判定的许可证；"
        f"确认其真实许可后写进 scripts/codegen/third_party_licenses.py 的 LICENSE_OVERRIDES，再重新生成"
    )


def load_rust() -> list[Component]:
    """按发布目标收集静态链接进可执行文件的 crate"""
    packages: dict[tuple[str, str], dict[str, Any]] = {}
    for supported in support.entries(Action.RELEASE):
        metadata = json.loads(cargo.licenses(supported.target))
        for package in metadata["packages"]:
            if _is_local(str(package["manifest_path"])):
                continue
            packages[str(package["name"]), str(package["version"])] = package

    components: list[Component] = []
    for (name, version), package in sorted(packages.items()):
        ids = license_ids(str(package.get("license") or ""))
        if not ids:
            raise _undeclared_error(name)
        root = Path(str(package["manifest_path"])).parent
        authors = ", ".join(str(author) for author in package.get("authors") or [])
        components.append(
            Component(
                name=name,
                version=version,
                licenses=ids,
                copyright=_copyright_of(root, authors or "（未声明）"),
                root=root,
            )
        )
    return components


def load_frontend() -> list[Component]:
    """收集打包进前端资源的生产依赖（`--prod` 已排除全部 devDependencies）"""
    buckets: dict[str, Any] = json.loads(pnpm.licenses())

    components: list[Component] = []
    for bucket, entries in buckets.items():
        for entry in entries:
            name = str(entry["name"])
            declaration = str(entry.get("license") or bucket)
            ids = license_ids(declaration)
            if not ids:
                override = LICENSE_OVERRIDES.get(name)
                if override is None:
                    raise _undeclared_error(name)
                ids = license_ids(override)
            paths = [Path(str(path)) for path in entry.get("paths") or []]
            root = next((path for path in paths if path.is_dir()), None)
            author = entry.get("author")
            fallback = author if isinstance(author, str) and author.strip() else "（未声明）"
            for version in entry.get("versions") or ["—"]:
                components.append(
                    Component(
                        name=name,
                        version=str(version),
                        licenses=ids,
                        copyright=_copyright_of(root, fallback),
                        root=root,
                    )
                )
    return components


def collect_texts(components: Sequence[Component]) -> dict[str, str]:
    """把各组件自带的许可正文汇总成 `{id: 正文}`，同一 id 只留第一次命中的一份"""
    texts: dict[str, str] = {}
    for component in components:
        for license_id, text in _scan_licenses(component.root).items():
            texts.setdefault(license_id, text)
    return texts


def _cell(text: str) -> str:
    """竖线会把表拆列，其余交给 markdown-it：`<https://…>` 与 `<mail@…>` 会被自动识别成链接"""
    return " ".join(text.replace("|", "\\|").split())


def _table(components: Sequence[Component]) -> list[str]:
    """渲染一张组件表（按组件名排序，版本随之稳定）"""
    rows = ["| 序号 | 组件 | 版本 | 许可证 | 版权 |", "| --- | --- | --- | --- | --- |"]
    for index, component in enumerate(sorted(components, key=lambda item: (item.name, item.version)), start=1):
        licenses = [f"[{license}](#{license.lower()})" for license in component.licenses]
        licenses_line = " + ".join(licenses)
        rows.append(
            f"| {index} | {_cell(component.name)} | {_cell(component.version)} "
            f"| {licenses_line} | {_cell(component.copyright)} |"
        )
    return rows


def _summary(components: Sequence[Component]) -> list[str]:
    """按主要许可证统计组件数"""
    counts = Counter(component.licenses[0] for component in components)
    rows = ["| 许可证 | 组件数 |", "| --- | ---: |"]
    for license_id, count in sorted(counts.items(), key=lambda item: (-item[1], item[0])):
        rows.append(f"| {license_id} | {count} |")
    return rows


def _text_order(license_ids_used: Sequence[str]) -> list[str]:
    """许可正文的排列：先按偏好顺序，再按字典序补上偏好表外的 id"""
    ordered = [license_id for license_id in LICENSE_PREFERENCE if license_id in license_ids_used]
    ordered.extend(sorted(set(license_ids_used) - set(ordered)))
    return ordered


def render(rust: Sequence[Component], frontend: Sequence[Component], texts: dict[str, str]) -> str:
    """拼出整份 Markdown"""
    all_components = [*rust, *frontend]
    used: list[str] = []
    for component in all_components:
        for license_id in component.licenses:
            if license_id not in used:
                used.append(license_id)

    lines = [
        "# 第三方许可证",
        "",
        "本应用包含下列第三方开源组件，各组件的著作权归其作者所有。",
        "",
        "",
        "",
        "## 许可类型汇总",
        "",
        *_summary(all_components),
        "",
        "注：同时适用多个许可证的组件，按其中较宽松的一项计入本表。",
        "",
        "## 组件清单",
        "",
        f"### Rust（{len(rust)} 个） {{#rust}}",
        "",
        *_table(rust),
        "",
        f"### Node.js（{len(frontend)} 个） {{#nodejs}}",
        "",
        *_table(frontend),
        "",
        "## 许可证内容",
        "",
        "同一许可证的正文只列一次；各组件的版权声明见上方清单。",
        "",
    ]

    for license_id in _text_order(used):
        text = texts.get(license_id)
        if text is None:
            raise NoticesError(
                f"许可证 {license_id!r} 的正文没能在任何依赖里找到；请按照 SPDX 补一条 `_SIGNATURES` 特征串，再重新生成"
            )
        lines.extend([f"### {license_id} {{#{license_id.lower()}}}", "", "```text", text, "```", ""])

    return "\n".join(lines).rstrip("\n") + "\n"


def generate() -> None:
    """收集前后端两侧的第三方组件，渲染并写入帮助窗口的第三方许可证"""
    rust = load_rust()
    frontend = load_frontend()
    info(f"第三方组件: Rust {len(rust)} 个，前端 {len(frontend)} 个")

    content = render(rust, frontend, collect_texts([*rust, *frontend]))
    THIRD_PARTY_NOTICES_PATH.parent.mkdir(parents=True, exist_ok=True)
    with THIRD_PARTY_NOTICES_PATH.open("w", encoding="utf-8", newline="\n") as file:
        _ = file.write(content)

    success(
        f"已生成 {str(THIRD_PARTY_NOTICES_PATH)!r}，大小为 {(THIRD_PARTY_NOTICES_PATH.stat().st_size / 1024):.3f} KB"
    )
