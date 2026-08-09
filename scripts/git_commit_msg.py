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
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Optional, Tuple


def run_git_command(cmd: List[str]) -> Tuple[bool, str]:
    """执行 Git 命令并返回结果"""
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            encoding='utf-8',
            errors='replace',
            check=False
        )
        if result.returncode != 0:
            return False, result.stderr.strip()
        return True, result.stdout.strip()
    except Exception as e:
        return False, str(e)


def tag_exists(tag: str) -> bool:
    """检查标签是否存在"""
    success, _ = run_git_command(["git", "rev-parse", tag])
    return success


def get_head_hash() -> Optional[str]:
    """获取 HEAD 的完整 hash"""
    success, output = run_git_command(["git", "rev-parse", "HEAD"])
    return output if success else None


def get_commit_range(start: Optional[str] = None, end: Optional[str] = None) -> List[Dict]:
    """
    获取指定范围的提交记录

    规则:
    - start 为 None: 从初始提交开始 (--root)
    - start 为标签: 不包含该标签 (tag..end)
    - end 为 None: 到 HEAD 结束 (包含 HEAD)
    - end 为标签: 包含该标签 (..tag)
    """
    # 构建范围
    if start is None:
        # 从 root 开始，包含所有
        if end is None:
            # 无 start 无 end: --root (所有提交)
            range_spec = "--root"
        else:
            # 无 start 有 end: end (包含 end)
            range_spec = end
    else:
        # 有 start: start..end (不包含 start，包含 end)
        if end is None:
            range_spec = f"{start}..HEAD"
        else:
            range_spec = f"{start}..{end}"

    # 使用空字符 \x00 分隔字段，使用 \x01 分隔不同提交
    format_str = "%H%x00%h%x00%ct%x00%ci%x00%s%x00%b%x01"
    cmd = [
        "git", "log", range_spec,
        f"--pretty=format:{format_str}",
        "--reverse"
    ]

    success, output = run_git_command(cmd)
    if not success or not output:
        return []

    commits = []
    for commit_block in output.rstrip('\x01').split('\x01'):
        if not commit_block:
            continue
        parts = commit_block.split('\x00')
        if len(parts) < 6:
            continue

        full_hash, short_hash, timestamp, full_time, subject, body = parts[:6]
        commits.append({
            'full_hash': full_hash,
            'short_hash': short_hash,
            'timestamp': int(timestamp),
            'full_time': full_time,
            'subject': subject.strip(),
            'body': body.strip()
        })

    return commits


def generate_markdown(commits: List[Dict], start: Optional[str], end: Optional[str]) -> str:
    """生成 Markdown 内容"""
    time_full = datetime.now().strftime("%Y-%m-%d %H:%M:%S %Z")
    _, branch = run_git_command(["git", "branch", "--show-current"])
    lines = []

    # 构建范围描述
    if start is None:
        start_desc = "root"
    else:
        start_desc = start

    if end is None:
        end_desc = "HEAD"
    else:
        end_desc = end

    # 标题
    lines.append("# 变更日志 (Change Log)\n")
    lines.append(f"**生成时间**: {time_full}\n")
    lines.append(f"**当前分支**: {branch}\n")
    lines.append(f"**版本范围**: {start_desc} → {end_desc}\n")
    lines.append(f"**提交总数**: {len(commits)}\n")
    lines.append("\n---\n")

    if not commits:
        lines.append("\n## ✅ 无新提交\n")
        lines.append(f"\n在范围 `{start_desc} → {end_desc}` 内没有新的变更。\n")
        return ''.join(lines)

    # 提交列表
    lines.append("\n## 📦 提交列表\n")

    digits = len(str(len(commits)))
    for idx, commit in enumerate(commits, start=1):
        seq = f"{idx:0{digits}d}"
        lines.append(f"\n### {seq}-{commit['subject']}\n")
        lines.append(f"> Hash: {commit['short_hash']}   At: {commit['full_time']}\n")
        if commit['body']:
            lines.append(commit['body'] + "\n")
        if idx != len(commits):
            lines.append("\n---\n")

    return ''.join(lines)


def build_filename(start: Optional[str], end: Optional[str], all_mode: bool = False) -> str:
    """
    根据范围构建文件名
    规则:
    - all_mode: root-to-HEAD.md
    - 有 start 有 end: start-to-end.md
    - 有 start 无 end: start-to-HEAD.md
    - 无 start 有 end: root-to-end.md
    - 无 start 无 end: 根据是否有标签决定
    """
    if all_mode:
        return "root-to-HEAD.md"

    # 清理标签名中的特殊字符
    def safe(s: str) -> str:
        return s.replace('/', '_').replace(' ', '_')

    if start is not None and end is not None:
        return f"{safe(start)}-to-{safe(end)}.md"
    elif start is not None and end is None:
        return f"{safe(start)}-to-HEAD.md"
    elif start is None and end is not None:
        return f"root-to-{safe(end)}.md"
    else:
        # 无参数情况: 检查是否有标签
        success, output = run_git_command(["git", "tag", "--sort=-creatordate"])
        if success and output:
            latest_tag = output.split('\n')[0]
            return f"{safe(latest_tag)}-to-HEAD.md"
        else:
            return "root-to-HEAD.md"


def save_changelog(content: str, start: Optional[str], end: Optional[str], all_mode: bool = False) -> Path:
    """保存 Change Log 到文件"""
    filename = build_filename(start, end, all_mode)
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


def main():
    # 检查是否在 Git 仓库中
    success, _ = run_git_command(["git", "rev-parse", "--git-dir"])
    if not success:
        print("❌ 错误：当前目录不是 Git 仓库")
        sys.exit(1)

    args = parse_args()

    # -a 模式：生成所有提交，忽略 -s/-e
    if args.all:
        print("📌 生成所有提交 (root → HEAD)")
        start = None
        end = None
        all_mode = True
    else:
        all_mode = False
        start = args.start
        end = args.end

        # 验证标签/commit 是否存在
        if start is not None:
            success, _ = run_git_command(["git", "rev-parse", start])
            if not success:
                print(f"❌ 错误：无法解析 '{start}'，请确认标签或 commit hash 是否存在")
                sys.exit(1)

        if end is not None:
            success, _ = run_git_command(["git", "rev-parse", end])
            if not success:
                print(f"❌ 错误：无法解析 '{end}'，请确认标签或 commit hash 是否存在")
                sys.exit(1)

        # 无参数时，自动检测最新标签
        if start is None and end is None:
            success, output = run_git_command(["git", "tag", "--sort=-creatordate"])
            if success and output:
                latest_tag = output.split('\n')[0]
                print(f"📌 使用最新标签: {latest_tag}")
                print(f"🔍 范围: {latest_tag}..HEAD (不包含 {latest_tag})")
                start = latest_tag
            else:
                print("📌 未找到任何标签，从初始提交开始")
                print("🔍 范围: --root (所有提交)")
            # end 保持 None = HEAD
        else:
            # 有参数时，打印范围信息
            start_desc = start if start else "root"
            end_desc = end if end else "HEAD"
            print(f"🔍 范围: {start_desc}..{end_desc}")

    commits = get_commit_range(start, end)
    print(f"📊 共找到 {len(commits)} 个提交")

    print("📝 生成 Markdown 内容...")
    content = generate_markdown(commits, start, end)

    filepath = save_changelog(content, start, end, all_mode)
    print(f"✅ Change Log 已保存到: {filepath}")


if __name__ == "__main__":
    main()
