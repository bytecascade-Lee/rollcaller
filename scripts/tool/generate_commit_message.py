#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
生成提交日志（Change Log）脚本

按 Git 标签 / commit 范围导出 Markdown，默认取「最新标签..HEAD」，仓库没有任何标签时从根提交开始。

用法:
    uv run ./scripts/tool/generate_commit_message.py                      # 最新标签..HEAD，无标签则 --root
    uv run ./scripts/tool/generate_commit_message.py -a                   # --root..HEAD（所有提交）
    uv run ./scripts/tool/generate_commit_message.py -s v0.1.0            # v0.1.0..HEAD（不含 v0.1.0）
    uv run ./scripts/tool/generate_commit_message.py -e v0.2.0            # --root..v0.2.0（含 v0.2.0）
    uv run ./scripts/tool/generate_commit_message.py -s v0.1.0 -e v0.2.0  # v0.1.0..v0.2.0（不含起点，含终点）
"""

import argparse
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Sequence

from cli import git
from common.constant.paths import DOCS_DIR
from common.logger import error, info, success
from util import fs
from util.str_format import display_width, padding

OUT_DIR = DOCS_DIR / "changes"


class ChangelogError(Exception):
    """仓库状态或范围参数不满足生成条件"""


@dataclass(frozen=True, slots=True)
class Commit:
    """一条 git log 记录：只承载值，渲染版式归本工具"""

    commit_hash: str
    commit_time: datetime
    subject: str = ""
    body: str = ""
    author: str = "unknown"
    email: str = "unknown"

    def __post_init__(self) -> None:
        if not self.commit_hash:
            raise ValueError("commit_hash 不能为空")

    @property
    def abbreviated_hash(self) -> str:
        """短哈希的宽度由 git 层约定，不在这里另立一份"""
        return self.commit_hash[git.ABBREVIATED_HASH_SLICE]


def _parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    """解析命令行参数"""
    parser = argparse.ArgumentParser(
        description="生成 git 变更日志 (Change Log)",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  %(prog)s                         # 最新标签..HEAD（无标签时 --root）
  %(prog)s -a                      # --root..HEAD (所有提交)
  %(prog)s -s v0.1.0               # v0.1.0..HEAD (不包含 v0.1.0)
  %(prog)s -e v0.2.0               # --root..v0.2.0 (包含 v0.2.0)
  %(prog)s -s v0.1.0 -e v0.2.0     # v0.1.0..v0.2.0 (不包含 v0.1.0, 包含 v0.2.0)
        """,
    )
    parser.add_argument(
        "-a", "--all",
        action="store_true",
        help="生成所有提交 (--root..HEAD)，忽略 -s/-e",
    )
    parser.add_argument(
        "-s", "--start",
        help="起始标签/commit (不包含自身)",
    )
    parser.add_argument(
        "-e", "--end",
        help="结束标签/commit (包含自身)",
    )
    return parser.parse_args(argv)


def _require_ref(ref: str, flag: str) -> str:
    """校验标签 / commit 是否存在，返回原值"""
    if not git.try_rev_parse(ref):
        raise ChangelogError(f"无法解析 {flag} 的 {ref!r}，请确认标签或 commit hash 是否存在")
    return ref


def _latest_tag() -> str | None:
    """按创建时间取最新标签；仓库没有标签时返回 None"""
    output = git.stdout(["tag", "--sort=-creatordate"])
    return output.splitlines()[0] if output else None


def _resolve_range(start: str | None, end: str | None, *, all_commits: bool) -> tuple[str, str]:
    """
    把 -a / -s / -e 归一成 (start, end)，start 不含、end 含：

    | 场景 | `--all` | `--start` | `--end` | 仓库标签 | 最终 start | 最终 end | 说明 |
    | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
    | 1 | 开启 | 任意（忽略） | 任意（忽略） | 任意 | `root` | `HEAD` | 最高优先级，直接覆盖 |
    | 2 | 关闭 | 提供 | 提供 | 任意 | 原值 | 原值 | 两侧都显式给出，只做存在性校验 |
    | 3 | 关闭 | 提供 | 未传 | 任意 | 原值 | `HEAD` | 只有起点，终点补 `HEAD` |
    | 4 | 关闭 | 未传 | 提供 | 任意 | `root` | 原值 | 只有终点，起点补初始提交 |
    | 5 | 关闭 | 未传 | 未传 | 至少一个标签 | `最新标签` | `HEAD` | 按创建时间取最新标签 |
    | 6 | 关闭 | 未传 | 未传 | 无标签 | `root` | `HEAD` | 降级为全量 |
    """
    if all_commits:
        return "root", "HEAD"

    if start is not None:
        _require_ref(start, "--start")
    if end is not None:
        _require_ref(end, "--end")

    if start is None and end is None:
        latest = _latest_tag()
        return (latest, "HEAD") if latest else ("root", "HEAD")

    return start or "root", end or "HEAD"


def _log_commits(start: str, end: str) -> list[Commit]:
    """读取 (start, end] 区间的提交，按时间升序"""
    if start == "root":
        range_spec = "--root" if end == "HEAD" else end
    else:
        range_spec = f"{start}..{end}"

    # %H=完整 hash  %ct=committer 时间(Unix 秒)  %s=标题  %b=正文
    # %an=作者名    %ae=作者邮箱
    # \x00 分隔字段，\x01 分隔提交
    output = git.stdout([
        "log",
        range_spec,
        "--pretty=format:%H%x00%ct%x00%s%x00%b%x00%an%x00%ae%x01",
        "--reverse",
    ])
    if not output:
        return []

    commits: list[Commit] = []
    for block in output.rstrip("\x01").split("\x01"):
        if not block:
            continue
        parts = block.split("\x00")
        if len(parts) < 6:
            continue
        commit_hash, ts, subject, body, author, email = parts[:6]

        commits.append(
            Commit(
                commit_hash=commit_hash.strip(),
                commit_time=datetime.fromtimestamp(int(ts), tz=timezone.utc),
                subject=subject,
                body=body,
                author=author,
                email=email,
            )
        )

    commits.sort(key=lambda item: item.commit_time)
    return commits


def _render_commit(commit: Commit) -> str:
    """把单个提交渲染为 changelog 中的一个条目"""
    left1 = f"Abbreviated hash: {commit.abbreviated_hash}"
    left2 = f"Author: {commit.author}"
    width = max(display_width(left1), display_width(left2))

    lines = [
        f"### {commit.subject}",
        f"> {padding(left1, width)}  |  At: {commit.commit_time.astimezone()}  ",
        f"> {padding(left2, width)}  |  Email: {commit.email}",
        commit.body.strip(),
    ]
    return "\n".join(lines)


def _render_markdown(commits: Sequence[Commit], start: str, end: str) -> str:
    """生成 Markdown 内容"""
    generated_at = datetime.now().astimezone().strftime("%Y-%m-%d %H:%M:%S %Z")
    lines = ["# 提交日志\n\n",
             f"**生成时间**: {generated_at}  \n",
             f"**当前分支**: {git.current_branch()}  \n",
             f"**版本范围**: {start} → {end}  \n",
             f"**提交总数**: {len(commits)}\n",
             "\n---\n"
             ]

    if not commits:
        lines.append("\n## 无提交\n")
        lines.append(f"\n在范围 `{start} -> {end}` 内没有新的变更。\n")
        return "".join(lines)

    lines.append("\n## 提交列表\n")
    for idx, commit in enumerate(commits, start=1):
        lines.append(_render_commit(commit))
        if idx != len(commits):
            lines.append("\n\n---\n\n")

    return "".join(lines)


def _save(content: str, start: str, end: str) -> Path:
    """保存 Change Log 到文件"""
    filename = f"{fs.sanitize_filename(start)}-to-{fs.sanitize_filename(end)}.md"
    path = OUT_DIR / filename
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    return path


def main(argv: Sequence[str] | None = None) -> int:
    args = _parse_args(argv)
    if not git.is_repo():
        raise ChangelogError("当前目录不在 Git 仓库")

    start, end = _resolve_range(args.start, args.end, all_commits=args.all)
    info(f"生成的范围是：{start} 至 {end}")

    commits = _log_commits(start, end)
    info(f"在该范围内共找到 {len(commits)} 个提交")

    info("准备生成 Markdown 内容......")
    content = _render_markdown(commits, start, end)
    info(f"已生成 {len(content)} 个字符的 Markdown")

    path = _save(content, start, end)
    success(f"Markdown 内容已保存到: {path}，大小为 {path.stat().st_size / 1024:.3f} KB")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (git.GitError, ChangelogError) as e:
        error(f"生成提交日志失败: {e}")
        sys.exit(1)
