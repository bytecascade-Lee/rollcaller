#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
导出 git 变更的 Markdown 快照：一个变更文件一个 .md，内含元数据 + 修改前全文 + unified diff。

变更范围两种口径：

    默认        工作区 vs HEAD（已 add 与未 add 的全部未提交改动）
    --from      从该版本比到工作区；给了 --to 则比到 --to 的那棵树（此模式不读磁盘）

范围可用位置参数收窄（文件 / 目录 / 通配符，一律按「当前工作目录」解释）：

    uv run scripts/tool/export_git_diff.py                           # 全部变更
    uv run scripts/tool/export_git_diff.py backend/src               # 目录：其下所有变更
    uv run scripts/tool/export_git_diff.py "backend/src/cmd/*.rs"     # 通配符
    uv run scripts/tool/export_git_diff.py a.rs b/c.ts               # 指定文件
    uv run scripts/tool/export_git_diff.py .                         # 当前目录下的全部变更

内容无实质性变化的文件不导出；重命名 / 复制（R / C）为路径变化，是实质性变更，照常导出。

用法：

    uv run scripts/tool/export_git_diff.py [选择器 ...] [选项]
    uv run scripts/tool/export_git_diff.py --dry-run            # 只看会导出什么
    uv run scripts/tool/export_git_diff.py --from v0.1.0        # v0.1.0 vs 工作区
    uv run scripts/tool/export_git_diff.py --from v0.1.0 --to v0.2.0
    uv run scripts/tool/export_git_diff.py --no-clean           # 保留上一轮的 md
"""

import argparse
import os
import re
import sys
from collections import Counter
from dataclasses import dataclass
from enum import Enum, auto
from pathlib import Path, PurePosixPath
from typing import Sequence

from cli import git
from common.constant.paths import PROJECT_BASE_DIR, DOCS_DIR
from common.logger import debug, error, info, success, warning
from util.fs import sanitize_filename
from util.str_format import display_width, padding

OUT_SUFFIX = ".md"  # 产物后缀：_out_name 与清理逻辑必须一致
OUT_DIR = DOCS_DIR / "diff"  # 固定输出位置，不可配置
META_GAP = 2  # 元数据标签与值之间的空格数
CTX_DEFAULT = 3  # unified diff 上下文行数
BINARY_SNIFF = 8192  # 二进制嗅探的字节数
LARGE_WARN = 1024 * 1024  # 超过此字节数只在日志里提一句，不阻断导出
FENCE_MIN = 3  # markdown 代码围栏的最小反引号数

STATUS_LABEL = {
    "M": "修改",
    "A": "新增",
    "D": "删除",
    "R": "重命名",
    "C": "复制",
    "T": "类型变更",
    "U": "冲突未解决",
}

# 代码围栏的语言标注；未收录的一律 text
LANG_BY_EXT = {
    "rs": "rust",
    "ts": "typescript",
    "js": "javascript",
    "mjs": "javascript",
    "cjs": "javascript",
    "svelte": "svelte",
    "py": "python",
    "json": "json",
    "json5": "json5",
    "jsonc": "jsonc",
    "yaml": "yaml",
    "yml": "yaml",
    "toml": "toml",
    "ini": "ini",
    "md": "markdown",
    "sql": "sql",
    "sh": "bash",
    "ps1": "powershell",
    "html": "html",
    "htm": "html",
    "css": "css",
    "xml": "xml",
    "svg": "xml",
    "drawio": "xml",
    "csv": "csv",
    "txt": "text",
}

_EOL_RE = re.compile(r"\r\n?")
_BACKTICK_RUN_RE = re.compile(r"`+")


class SkipError(Exception):
    """该文件无法导出（二进制 / 非 UTF-8 / 读取失败），原因在 str(e)"""


class UsageError(Exception):
    """参数组合不成立，退出码 2"""


class Outcome(Enum):
    """单个变更的导出结果"""

    WRITTEN = auto()  # 已写盘（dry-run 下计为「会写」）
    SKIPPED = auto()  # 取数失败 / 两侧都没内容，已记日志
    EOL_ONLY = auto()  # 仅行尾差异，按「无实质变更」跳过


@dataclass
class Change:
    """一条 `git diff --name-status` 记录"""

    code: str  # 归一化后的单字母状态
    raw: str  # git 原始状态码，重命名/复制可能是 R100 / C75
    old_path: str | None  # 重命名/复制的旧路径（仓库相对，'/' 分隔）
    new_path: str  # 当前路径

    @property
    def is_rename(self) -> bool:
        """重命名或复制：旧路径与新路径不同，两棵树要各取一边"""
        return self.old_path is not None


@dataclass
class Snapshot:
    """一个文件的取数结果"""

    before: str | None  # None = 该版本不存在此文件
    after: str | None
    before_size: int | None
    after_size: int | None
    patch: str
    added: int
    deleted: int


@dataclass(frozen=True, slots=True)
class Scope:
    """一次导出的对比范围"""

    diff_args: tuple[str, ...]  # 传给 git diff 的位置参数：HEAD / <from> / <from> <to>
    before_ref: str
    after_ref: str | None  # None 表示「修改后」取自工作区
    labels: tuple[str, str]  # 渲染标题用的两侧名称

    @property
    def base_label(self) -> str:
        """命令行等价的 base 描述，如 `v0.1.0 v0.2.0`"""
        return " ".join(self.diff_args)


def _blob_or_warn(ref: str, path: str) -> bytes | None:
    """取 `<ref>:<path>` 的原始内容；取不到时告警并返回 None（原因见 DEBUG 日志）"""
    data = git.show_blob(ref, path)
    if data is None:
        warning(f"{ref}:{path} 取不到内容，按「不存在」处理")
    return data


def _read_worktree(path: str) -> bytes | None:
    """读工作区文件；已被删掉时返回 None"""
    try:
        return (PROJECT_BASE_DIR / path).read_bytes()
    except FileNotFoundError:
        return None
    except OSError as e:
        raise SkipError(f"读取工作区文件失败: {e}")


def _as_text(data: bytes, where: str) -> str:
    """字节 -> 文本；二进制或非 UTF-8 抛 SkipError"""
    if b"\x00" in data[:BINARY_SNIFF]:
        raise SkipError(f"{where} 疑似二进制（前 {BINARY_SNIFF} 字节含 NUL）")
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError as e:
        raise SkipError(f"{where} 不是合法 UTF-8（{e}）")


def _same_content(before: str, after: str) -> bool:
    """两侧内容是否无实质性差异"""
    return _EOL_RE.sub("\n", before) == _EOL_RE.sub("\n", after)


def _list_changes(diff_args: Sequence[str], selectors: Sequence[str]) -> list[Change]:
    """枚举变更"""
    args = ["diff", *diff_args, "--name-status", "-z", "-M"]
    if selectors:
        args += ["--", *selectors]
    out = git.stdout(args, cwd=PROJECT_BASE_DIR)
    if not out:
        return []

    fields = out.split("\0")
    changes: list[Change] = []
    i = 0
    # 末位是结尾 NUL 产生的空串，故只走到 len(fields) - 1
    while i < len(fields) - 1:
        raw = fields[i]
        if not raw:
            i += 1
            continue
        if raw[0] in ("R", "C"):
            if i + 2 >= len(fields):
                warning(f"状态行不完整，跳过: {raw!r}")
                break
            old_path, new_path = fields[i + 1], fields[i + 2]
            i += 3
        else:
            old_path, new_path = None, fields[i + 1]
            i += 2

        for p in (old_path, new_path):
            if p and "\ufffd" in p:
                warning(f"路径含无法解码的字节，该文件按原样处理: {p}")
        # R/C 的码是 R100 这种，单字母取首；其余多字母码（如 AD）按 porcelain 约定取末位（工作区侧）
        code = raw[0] if raw[0] in ("R", "C") else raw[-1]
        changes.append(Change(code=code, raw=raw, old_path=old_path, new_path=new_path))

    return changes


def _normalize_selectors(raw: Sequence[str]) -> list[str]:
    """把位置参数归一化成相对仓库根的路径，作为 git pathspec 使用。

    - 相对路径按「当前工作目录」解释，与用户敲命令的位置一致
    - 不做存在性检查：已从工作区删除的文件也必须能作为 pathspec 命中其变更
    - 通配符不做本地展开，交给 git 自己匹配（git pathspec 支持 * 与 ?，
      且天然跳过 .gitignore 的目录，不会去遍历 backend/target、node_modules）
    """
    cwd = Path.cwd()
    normalized: list[str] = []
    for item in raw:
        target = Path(item)
        if not target.is_absolute():
            target = cwd / target
        try:
            rel = os.path.relpath(target, PROJECT_BASE_DIR)
        except ValueError:  # Windows 跨盘符
            warning(f"路径与仓库不在同一盘符，忽略: {item}")
            continue
        rel = Path(rel).as_posix()
        if rel == ".." or rel.startswith("../"):
            warning(f"路径在仓库之外，忽略: {item}")
            continue
        normalized.append(rel)
    return normalized


def _patch_of(diff_args: Sequence[str], paths: Sequence[str], context: int) -> str:
    """取本文件的 unified diff；重命名要同时传新旧路径，否则 git 会退化成「新增+删除」"""
    return git.stdout(
        ["diff", *diff_args, f"-U{context}", "--no-color", "-M", "--", *paths],
        cwd=PROJECT_BASE_DIR,
    )


def _count_untracked() -> int:
    """未跟踪文件数（只用于日志说明它们被忽略了）"""
    try:
        out = git.stdout(["ls-files", "--others", "--exclude-standard", "-z"], cwd=PROJECT_BASE_DIR)
    except git.GitError:
        return 0
    return len([f for f in out.split("\0") if f]) if out else 0


def _before_bytes(change: Change, before_ref: str) -> bytes | None:
    if change.code == "A":
        return None  # 新增：该版本本来就没有
    path = change.old_path if change.is_rename else change.new_path
    return _blob_or_warn(before_ref, path)


def _after_bytes(change: Change, after_ref: str | None) -> bytes | None:
    if change.code == "D":
        return None  # 删除：工作区（或 to 树）里没有了
    if after_ref is None:
        return _read_worktree(change.new_path)
    return _blob_or_warn(after_ref, change.new_path)


# 渲染（Markdown）
def _status_text(change: Change) -> str:
    label = STATUS_LABEL.get(change.code, "未知")
    if change.raw != change.code:
        return f"{change.code}（{label}，git 原始码 {change.raw}）"
    return f"{change.code}（{label}）"


def _size_text(before: int | None, after: int | None) -> str:
    b = "不存在" if before is None else f"{before:,} B"
    a = "不存在" if after is None else f"{after:,} B"
    return f"修改前 {b} -> 修改后 {a}"


def _count_patch_lines(patch: str) -> tuple[int, int]:
    """从补丁正文数增删行（+++/--- 表头与 `\\ No newline` 不计），与内嵌补丁必然一致"""
    added = deleted = 0
    for line in patch.splitlines():
        if line.startswith("+") and not line.startswith("+++"):
            added += 1
        elif line.startswith("-") and not line.startswith("---"):
            deleted += 1
    return added, deleted


def _meta_block(rows: Sequence[tuple[str, str]]) -> list[str]:
    """元数据块：标签按显示宽度右补空格对齐，行尾补两个空格触发 markdown 硬换行。"""
    width = max(display_width(label) for label, _ in rows)
    return [f"{padding(label, width)}{' ' * META_GAP}{value}  " for label, value in rows]


def _fence(text: str) -> str:
    """围栏长度 = 正文里最长反引号串 + 1（至少 FENCE_MIN），防止被正文中的 ``` 提前闭合。"""
    longest = max((len(m.group()) for m in _BACKTICK_RUN_RE.finditer(text)), default=0)
    return "`" * max(FENCE_MIN, longest + 1)


def _lang_of(path: str) -> str:
    """按扩展名推断代码围栏的语言标注"""
    ext = PurePosixPath(path).suffix.lstrip(".").lower()
    return LANG_BY_EXT.get(ext, "text")


def _code_block(text: str | None, lang: str, missing: str) -> list[str]:
    """一个 fenced code block；text 为 None 时输出 missing 占位说明"""
    if text is None:
        return [missing]
    if text == "":
        return ["（空文件）"]
    body = text if text.endswith("\n") else text + "\n"
    fence = _fence(body)
    return [f"{fence}{lang}", body + fence]


def _render(
    change: Change,
    snap: Snapshot,
    labels: tuple[str, str],
    base_label: str,
    context: int,
) -> str:
    rows: list[tuple[str, str]] = [
        ("文件", f"`{change.new_path}`"),
        ("磁盘位置", f"`{PROJECT_BASE_DIR / change.new_path}`"),
        ("变更状态", _status_text(change)),
    ]
    if change.is_rename:
        rename_label = "重命名" if change.code == "R" else "复制"
        rows.append((rename_label, f"`{change.old_path}` -> `{change.new_path}`"))
    rows.append(("大小", _size_text(snap.before_size, snap.after_size)))
    rows.append(("变更行数", f"+{snap.added} / -{snap.deleted}"))

    lines: list[str] = []
    lines += _meta_block(rows)
    lines.append("")
    lines.append(f"## 修改前（{labels[0]}）")
    lines.append("")
    lines += _code_block(snap.before, _lang_of(change.new_path), "（该版本不存在此文件）")
    lines.append("")
    lines.append(f"## unified diff（`git diff {base_label} -U{context}`）")
    lines.append("")
    lines += _code_block(snap.patch if snap.patch else None, "diff", "（无行级差异）")
    lines.append("")

    text = "\n".join(lines)
    return text if text.endswith("\n") else text + "\n"


def _out_name(change: Change) -> str:
    # 路径里的非法字符（: * ? 等，Linux 侧仓库可能存在）与空白一并清洗
    stem = sanitize_filename("__".join(change.new_path.split("/")))
    return f"{stem}.{change.code}{OUT_SUFFIX}"


# 输出目录

def _clean_out_dir(out_dir: Path) -> int:
    """清掉上一轮结果：只删本目录下直接的产物文件，不递归、不删目录、不跟随符号链接"""
    if not out_dir.is_dir():
        return 0

    removed = 0
    for item in sorted(out_dir.iterdir()):
        if not item.is_file() or item.suffix.lower() != OUT_SUFFIX:
            continue
        try:
            item.unlink()
        except OSError as e:
            warning(f"清理失败: {item}（{e}）")
            continue
        removed += 1
        debug(f"清理旧结果: {item.name}")
    return removed


# 入口

def _parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="导出 git 变更：每个文件一个 md（元数据 + 修改前全文 + unified diff）",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "位置参数（文件 / 目录 / 通配符）按当前工作目录解释，通配符交给 git 匹配；\n"
            "不给位置参数就是全部变更。\n"
            "输出文件名示例：backend__src__repo__record_repo.rs.M.md\n"
            "状态后缀：M 修改 / A 新增 / D 删除 / R 重命名；内容仅行尾不同的不导出。"
        ),
    )
    parser.add_argument(
        "selectors",
        nargs="*",
        metavar="PATH",
        help="只导出这些文件/目录下的变更；支持通配符，不传则导出全部",
    )
    parser.add_argument(
        "--from",
        dest="from_rev",
        metavar="REV",
        help="起始版本（默认 HEAD）；单独给出时为「该版本 vs 工作区」",
    )
    parser.add_argument(
        "--to",
        dest="to_rev",
        metavar="REV",
        help="结束版本；给出后「修改后」取自该版本而非工作区（须与 --from 同时给出）",
    )
    parser.add_argument(
        "--context",
        type=int,
        default=CTX_DEFAULT,
        metavar="N",
        help=f"unified diff 上下文行数（默认 {CTX_DEFAULT}）",
    )
    parser.add_argument("--no-clean", action="store_true", help="保留输出目录里上一轮的产物")
    parser.add_argument("--dry-run", action="store_true", help="只打印将要写入什么，不落盘")
    return parser.parse_args(argv)


def _resolve_scope(args: argparse.Namespace) -> Scope:
    """把 --from / --to 归一成一次对比范围"""
    if args.to_rev and not args.from_rev:
        raise UsageError("--to 必须与 --from 同时给出（只有 --to 无法确定起点）")
    if args.context < 0:
        raise UsageError(f"--context 不能为负: {args.context}")

    if args.from_rev and args.to_rev:
        return Scope(
            diff_args=(args.from_rev, args.to_rev),
            before_ref=args.from_rev,
            after_ref=args.to_rev,
            labels=(args.from_rev, args.to_rev),
        )
    if args.from_rev:
        return Scope(
            diff_args=(args.from_rev,),
            before_ref=args.from_rev,
            after_ref=None,
            labels=(args.from_rev, "工作区"),
        )
    return Scope(
        diff_args=("HEAD",),
        before_ref="HEAD",
        after_ref=None,
        labels=("HEAD", "工作区"),
    )


def _plan(changes: Sequence[Change]) -> tuple[list[tuple[Change, str]], list[tuple[str, list[str]]]]:
    """预先分配输出名，返回（可导出的变更及其文件名，输出名冲突的分组）

    冲突必须在写盘前一次性暴露：边写边发现只会留下半套产物。
    """
    by_name: dict[str, list[Change]] = {}
    for change in changes:
        by_name.setdefault(_out_name(change), []).append(change)

    planned: list[tuple[Change, str]] = []
    conflicts: list[tuple[str, list[str]]] = []
    for name, group in by_name.items():
        if len(group) == 1:
            planned.append((group[0], name))
        else:
            conflicts.append((name, [item.new_path for item in group]))
    return planned, conflicts


def _export_one(
    change: Change,
    name: str,
    scope: Scope,
    *,
    context: int,
    out_dir: Path,
    dry_run: bool,
) -> Outcome:
    """导出一个变更；失败原因已记日志，由调用方按 Outcome 计数"""
    try:
        before_bytes = _before_bytes(change, scope.before_ref)
        after_bytes = _after_bytes(change, scope.after_ref)
        where_before = f"{scope.before_ref}:{change.old_path or change.new_path}"
        before = None if before_bytes is None else _as_text(before_bytes, where_before)
        after = None if after_bytes is None else _as_text(after_bytes, change.new_path)
    except (SkipError, git.GitError) as e:
        warning(f"跳过: {change.new_path}（{e}）")
        return Outcome.SKIPPED

    if before is None and after is None:
        warning(f"跳过: {change.new_path}（两侧都取不到内容，git 状态 {change.raw}）")
        return Outcome.SKIPPED

    # 内容口径过滤：仅行尾不同（CRLF ↔ LF）的修改不导出。重命名/复制不适用。
    if (
        change.code == "M"
        and before is not None
        and after is not None
        and _same_content(before, after)
    ):
        info(f"跳过: {change.new_path}（仅行尾差异，内容无实质变化）")
        return Outcome.EOL_ONLY

    paths = (
        [p for p in (change.old_path, change.new_path) if p]
        if change.is_rename
        else [change.new_path]
    )
    try:
        patch = _patch_of(scope.diff_args, paths, context)
    except git.GitError as e:
        warning(f"取补丁失败，只导出全文: {change.new_path}（{e}）")
        patch = ""

    added, deleted = _count_patch_lines(patch)
    snap = Snapshot(
        before=before,
        after=after,
        before_size=None if before_bytes is None else len(before_bytes),
        after_size=None if after_bytes is None else len(after_bytes),
        patch=patch,
        added=added,
        deleted=deleted,
    )

    biggest = max([s for s in (snap.before_size, snap.after_size) if s is not None], default=0)
    if biggest > LARGE_WARN:
        warning(f"文件较大（{biggest:,} B），导出内容将占用较多空间: {change.new_path}")

    text = _render(change, snap, scope.labels, scope.base_label, context)

    if dry_run:
        info(f"[dry-run] {change.raw} {change.new_path} -> {name}（+{added}/-{deleted}）")
        return Outcome.WRITTEN

    target = out_dir / name
    # newline="" 关掉换行翻译，否则正文里的行尾会被二次加工（embedded CRLF 亦受影响）
    with target.open("w", encoding="utf-8", newline="") as f:
        f.write(text)
    info(
        f"已导出 {change.raw} {change.new_path} -> {name}"
        f"（+{added}/-{deleted}, {_size_text(snap.before_size, snap.after_size)}）"
    )
    return Outcome.WRITTEN


def _report(changes: Sequence[Change], stats: Counter[Outcome], out_dir: Path) -> None:
    """汇总本轮结果"""
    untracked = _count_untracked()
    same_content = stats[Outcome.EOL_ONLY]
    same_note = f"（其中仅行尾差异 {same_content} 个）" if same_content else ""
    success(
        f"完成：导出 {stats[Outcome.WRITTEN]} 个"
        f" / 跳过 {stats[Outcome.SKIPPED] + same_content} 个{same_note}"
        f" / 未跟踪忽略 {untracked} 个 / 变更总数 {len(changes)} → {out_dir}"
    )


def main(argv: Sequence[str] | None = None) -> int:
    args = _parse_args(argv)
    try:
        scope = _resolve_scope(args)
    except UsageError as e:
        error(str(e))
        return 2

    out_dir = OUT_DIR
    selectors = _normalize_selectors(args.selectors)

    info(f"项目根: {PROJECT_BASE_DIR}")
    info(f"对比: {scope.base_label}" + ("" if scope.after_ref else "（结束侧取工作区）"))
    if selectors:
        info(f"范围: {' '.join(selectors)}")
    info(f"输出: {out_dir}{'（dry-run，不落盘）' if args.dry_run else ''}")

    try:
        changes = _list_changes(scope.diff_args, selectors)
    except git.GitError as e:
        error(str(e))
        return 1

    if not changes:
        scope_note = f" {' '.join(selectors)}" if selectors else ""
        info(f"没有变更（{scope.base_label}{scope_note}）")
        return 0

    planned, conflicts = _plan(changes)
    if conflicts:
        for name, paths in conflicts:
            error(f"输出名冲突: {' / '.join(paths)} 都映射到 {name}（请改用镜像目录结构或分批导出）")
        error(f"共 {len(conflicts)} 组输出名冲突，本次不写出任何文件")
        return 1

    if not args.dry_run:
        if not args.no_clean:
            removed = _clean_out_dir(out_dir)
            if removed:
                info(f"已清理 {removed} 个旧产物")
        out_dir.mkdir(parents=True, exist_ok=True)

    stats: Counter[Outcome] = Counter()
    for change, name in planned:
        outcome = _export_one(
            change,
            name,
            scope,
            context=args.context,
            out_dir=out_dir,
            dry_run=args.dry_run,
        )
        stats[outcome] += 1

    _report(changes, stats, out_dir)
    return 0


if __name__ == "__main__":
    sys.exit(main())
