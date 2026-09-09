#!/usr/bin/env python3
"""
生成 Change Log 脚本
根据 Git 标签状态生成 Markdown 格式的变更日志

用法:
    python git_commit_msg.py                # 默认: latest_tag..HEAD 或 --root
    python git_commit_msg.py -a             # --root..HEAD (所有提交)
    python git_commit_msg.py -s v0.1.0      # v0.1.0..HEAD (不包含 v0.1.0)
    python git_commit_msg.py -e v0.2.0      # --root..v0.2.0 (包含 root, 包含 v0.2.0)
    python git_commit_msg.py -s v0.1.0 -e v0.2.0  # v0.1.0..v0.2.0 (不包含 v0.1.0, 包含 v0.2.0)
"""

import argparse
import sys
from datetime import datetime
from pathlib import Path
from typing import List, Optional, Tuple

from common import git


def generate_markdown(commits: List[git.CommitInfo], start: str, end: str) -> str:
    """生成 Markdown 内容"""
    iso_time = datetime.now().strftime("%Y-%m-%d %H:%M:%S %Z")
    branch = git.get_branch()
    lines = ["# 提交日志\n",
             f"**生成时间**: {iso_time}\n",
             f"**当前分支**: {branch}\n",
             f"**版本范围**: {start} → {end}\n",
             f"**提交总数**: {len(commits)}\n",
             "\n---\n"
             ]

    if not commits:
        lines.append("\n## ✅ 无新提交\n")
        lines.append(f"\n在范围 `{start} → {end}` 内没有新的变更。\n")
        return ''.join(lines)

    # 提交列表
    lines.append("\n## 📦 提交列表\n")

    digits = len(str(len(commits)))
    for idx, commit in enumerate(commits, start=1):
        seq = f"{idx:0{digits}d}"
        lines.append(f"\n### {seq}-{commit.subject}\n")
        lines.append(f"> Hash: {commit.short_hash}   At: {commit.datatime}\n")
        if commit.body:
            lines.append(commit.body + "\n")
        if idx != len(commits):
            lines.append("\n---\n")

    return ''.join(lines)


def save_changelog(content: str, start: str, end: str) -> Path:
    """保存 Change Log 到文件"""
    filename = f"{start.replace('/', '_').replace(' ', '_')}-to-{end.replace('/', '_').replace(' ', '_')}.md"
    filepath = Path.cwd() / "docs/changes" / filename
    filepath.parent.mkdir(parents=True, exist_ok=True)
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    return filepath


def parse_args():
    """解析命令行参数"""
    parser = argparse.ArgumentParser(
        description="生成 Git 变更日志 (Change Log)",
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
        '-a', '--all',
        action='store_true',
        help='生成所有提交 (--root..HEAD)，忽略 -s/-e'
    )
    parser.add_argument(
        '-s', '--start',
        help='起始标签/commit (不包含自身)'
    )
    parser.add_argument(
        '-e', '--end',
        help='结束标签/commit (包含自身)'
    )
    return parser.parse_args()


def parse_start_end_all(start: Optional[str], end: Optional[str], all: Optional[bool]) -> Tuple[str, str]:
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
    all: bool = True if all else False
    # 如果有参，验证 tags/commit 是否存在
    if start is not None:
        success = git.commit_exists(start)
        if not success:
            print(f"❌ 错误：无法解析 '{start}'，请确认标签或 commit hash 是否存在")
            sys.exit(1)

    if end is not None:
        success = git.commit_exists(end)
        if not success:
            print(f"❌ 错误：无法解析 '{end}'，请确认标签或 commit hash 是否存在")
            sys.exit(1)

    # 如果无参，自动检测最新标签
    if start is None and end is None and not all:
        output = git.git(["tag", "--sort=-creatordate"])
        if output:
            latest_tag = output.split('\n')[0]
            start = latest_tag
            end = "HEAD"
            print(f"📌 使"
                  f"用最新标签: {latest_tag}")
        else:
            start = "root"
            end = "HEAD"
            print("📌 未找到任何标签，从初始提交开始")

    if start is not None and end is None:
        end = "HEAD"

    if start is None and end is not None:
        start = "root"

    # 如果全部
    if all:
        print("📌 生成所有提交 (root → HEAD)")
        start = "root"
        end = "HEAD"

    return start, end


def main():
    # 检查是否在 Git 仓库中
    success = git.is_repo()
    if not success:
        print("❌ 错误：当前目录不是 Git 仓库")
        sys.exit(1)
    args = parse_args()
    start, end = parse_start_end_all(args.start, args.end, args.all)

    print(f"🔍 范围: {start}..{end}")
    commits = git.get_commit_range(start, end)
    print(f"📊 共找到 {len(commits)} 个提交")

    print("📝 生成 Markdown 内容...")
    content = generate_markdown(commits, start, end)

    filepath = save_changelog(content, start, end)
    print(f"✅ Change Log 已保存到: {filepath}")


if __name__ == "__main__":
    main()
