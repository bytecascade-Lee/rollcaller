#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
生成 Change Log 脚本
根据 Git 标签状态生成 Markdown 格式的变更日志

用法:
    uv run ./scripts/codegen/git_commit_msg.py                      # 默认: latest_tag..HEAD 或 --root
    uv run ./scripts/codegen/git_commit_msg.py -a                   # --root..HEAD (所有提交)
    uv run ./scripts/codegen/git_commit_msg.py -s v0.1.0            # v0.1.0..HEAD (不包含 v0.1.0)
    uv run ./scripts/codegen/git_commit_msg.py -e v0.2.0            # --root..v0.2.0 (包含 root, 包含 v0.2.0)
    uv run ./scripts/codegen/git_commit_msg.py -s v0.1.0 -e v0.2.0  # v0.1.0..v0.2.0 (不包含 v0.1.0, 包含 v0.2.0)
"""

import argparse
from datetime import datetime, timezone
from pathlib import Path
from typing import List, Optional, Tuple

import util
from cli import git
from cli.git import stdout
from common.constant.paths import DOCS_DIR, PROJECT_BASE_DIR
from common.entity import commit_info
from common.entity.commit_info import CommitInfo
from common.logger import info, success
from util.str_format import display_width, padding


def parse_args():
    """解析命令行参数"""
    parser = argparse.ArgumentParser(
        description="生成 git 变更日志 (Change Log)",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  %(prog)s                         # latest_tag..HEAD 或 --root
  %(prog)s -a                      # --root..HEAD (所有提交)
  %(prog)s -s v0.1.0               # v0.1.0..HEAD (不包含 v0.1.0)
  %(prog)s -e v0.2.0               # --root..v0.2.0 (包含 v0.2.0)
  %(prog)s -s v0.1.0 -e v0.2.0     # v0.1.0..v0.2.0 (不包含 v0.1.0, 包含 v0.2.0)
        """
    )
    parser.add_argument(
        "-a", "--all",
        action="store_true",
        help="生成所有提交 (--root..HEAD)，忽略 -s/-e"
    )
    parser.add_argument(
        "-s", "--start",
        help="起始标签/commit (不包含自身)"
    )
    parser.add_argument(
        "-e", "--end",
        help="结束标签/commit (包含自身)"
    )
    return parser.parse_args()


def parse_start_end_all(start: Optional[str], end: Optional[str], is_all: Optional[bool]) -> Tuple[str, str]:
    """
    | 场景 | `--all` 参数 | `--start` 入参 | `--end` 入参 | 仓库标签状态 | **最终 `start` 值** | **最终 `end` 值** | 触发逻辑与说明 |
    | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
    | **1** | `True` (开启) | 任意（忽略） | 任意（忽略） | 任意 | `"root"` | `"HEAD"` | **最高优先级**。直接覆盖所有传入值，强制输出全部提交 |
    | **2** | `False` (关闭) | 提供（如 `v1.0`） | 提供（如 `v2.0`） | 任意（均校验通过） | `"v1.0"` | `"v2.0"` | 两者显式传入，校验通过后**原样保留**，不自动补全 |
    | **3** | `False` (关闭) | 提供（如 `v1.0`） | `None` (未传) | 任意（校验通过） | `"v1.0"` | `"HEAD"` | 只有起点，终点自动补全为 `HEAD` |
    | **4** | `False` (关闭) | `None` (未传) | 提供（如 `v2.0`） | 任意（校验通过） | `"root"` | `"v2.0"` | 只有终点，起点自动补全为 `root`（初始提交） |
    | **5** | `False` (关闭) | `None` (未传) | `None` (未传) | **存在**至少一个标签 | `最新标签名`（如 `v1.5`） | `"HEAD"` | 智能检测：按创建时间取最新标签作为起点 |
    | **6** | `False` (关闭) | `None` (未传) | `None` (未传) | **不存在**任何标签 | `"root"` | `"HEAD"` | 降级处理：无标签时从根提交开始 |
    """
    # 如果有参，验证 tags/commit 是否存在
    if start is not None and not git.try_rev_parse(start):
        util.fail(f"无法解析 `{start}`，请确认标签或 commit hash 是否存在")

    if end is not None and not git.try_rev_parse(end):
        util.fail(f"无法解析 `{end}`，请确认标签或 commit hash 是否存在")

    # 如果无参，自动检测最新标签
    if start is None and end is None and not is_all:
        output = git.stdout(["tag", "--sort=-creatordate"])
        if output:
            latest_tag = output.split("\n")[0]
            start = latest_tag
            end = "HEAD"
        else:
            start = "root"
            end = "HEAD"

    if start is not None and end is None:
        end = "HEAD"

    if start is None and end is not None:
        start = "root"

    # 如果全部
    if is_all:
        start = "root"
        end = "HEAD"

    return start, end


def _render_commit(commit: commit_info.CommitInfo) -> str:
    """把单个提交渲染为 changelog 中的一个条目（版式归本工具，实体只提供值）"""
    left1 = f"Abbreviated hash: {commit.abbreviated_hash}"
    left2 = f"Author: {commit.author}"
    width = max(display_width(left1), display_width(left2))

    lines = [
        f"### {commit.subject}",
        f"> {padding(left1, width)}  |  At: {commit.commit_time}  ",
        f"> {padding(left2, width)}  |  Email: {commit.email}",
        commit.body.strip(),
    ]
    return "\n".join(lines)


def generate_markdown(commits: List[commit_info.CommitInfo], start: str, end: str) -> str:
    """生成 Markdown 内容"""
    iso_time = datetime.now().strftime("%Y-%m-%d %H:%M:%S %Z")
    branch = git.current_branch()
    lines = ["# 提交日志\n\n",
             f"**生成时间**: {iso_time}  \n",
             f"**当前分支**: {branch}  \n",
             f"**版本范围**: {start} → {end}  \n",
             f"**提交总数**: {len(commits)}\n",
             "\n---\n"
             ]

    if not commits:
        lines.append("\n## 无提交\n")
        lines.append(f"\n在范围 `{start} -> {end}` 内没有新的变更。\n")
        return "".join(lines)

    # 提交列表
    lines.append("\n## 提交列表\n")

    for idx, commit in enumerate(commits, start=1):
        lines.append(_render_commit(commit))
        if idx != len(commits):
            lines.append("\n\n---\n\n")

    return "".join(lines)


def save_changelog(content: str, start: str, end: str) -> Path:
    """保存 Change Log 到文件"""
    filename = f"{util.sanitize_filename(start)}-to-{util.sanitize_filename(end)}.md"
    filepath = DOCS_DIR / "changes" / filename
    filepath.parent.mkdir(parents=True, exist_ok=True)
    with open(filepath, "w", encoding="utf-8") as f:
        f.write(content)
    return filepath


def get_commit_range(start: str, end: str, cwd: Path = PROJECT_BASE_DIR) -> list[CommitInfo]:
    """
    获取指定范围的提交记录。

    Args:
        start: 起始 commit/tag（不包含），"root" 表示从 root 开始
        end: 结束 commit/tag（包含），"HEAD" 表示到 HEAD
        cwd: 当前工作目录

    Returns:
        提交列表，按时间升序排列。每个 CommitInfo 包含：
            commit_hash:      完整 hash
            abbreviated_hash: 短 hash（派生）
            commit_time:      带本地时区的 datetime
            subject:          提交标题
            body:             提交正文
            author:           作者名
            email:            作者邮箱
    """
    # 构建范围字符串
    if start == "root" and end == "HEAD":
        range_spec = "--root"
    elif start == "root":
        range_spec = end
    else:
        range_spec = f"{start}..{end}"

    # %H=完整 hash  %ct=committer 时间(Unix 秒)  %s=标题  %b=正文
    # %an=作者名    %ae=作者邮箱
    # \x00 分隔字段，\x01 分隔提交
    format_str = "%H%x00%ct%x00%s%x00%b%x00%an%x00%ae%x01"
    cmd = [
        "log",
        range_spec,
        f"--pretty=format:{format_str}",
        "--reverse",
    ]

    output = stdout(cmd, cwd=cwd)
    if not output:
        return []

    commits: list[CommitInfo] = []
    for block in output.rstrip("\x01").split("\x01"):
        if not block:
            continue
        parts = block.split("\x00")
        if len(parts) < 6:
            continue
        commit_hash, ts, subject, body, author, email = parts[:6]

        commits.append(
            CommitInfo(
                commit_hash=commit_hash.strip(),
                commit_time=datetime.fromtimestamp(int(ts), tz=timezone.utc),
                subject=subject,
                body=body,
                author=author,
                email=email,
            )
        )

    commits.sort(key=lambda x: x.commit_time)
    return commits


def main():
    if not git.is_repo():
        util.fail("当前目录不在 Git 仓库")
    args = parse_args()
    start, end = parse_start_end_all(args.start, args.end, args.all)
    info(f"生成的范围是：{start} 至 {end}")
    commits = get_commit_range(start, end)
    info(f"在该范围内共找到 {len(commits)} 个提交")

    info("准备生成 Markdown 内容......")
    content = generate_markdown(commits, start, end)
    info(f"已生成 {len(content)} 行 Markdown")

    filepath = save_changelog(content, start, end)
    success(f"Markdown 内容已保存到: {filepath}，大小为 {filepath.stat().st_size / 1024:.3f} KB")


if __name__ == "__main__":
    main()
