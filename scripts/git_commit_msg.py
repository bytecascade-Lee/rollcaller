#!/usr/bin/env python3
"""
生成 Change Log 脚本
根据 Git 标签状态生成 Markdown 格式的变更日志
"""

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


def get_latest_tag() -> Optional[str]:
    """获取最新的附注标签（按创建时间排序）"""
    success, output = run_git_command(["git", "tag", "--sort=-creatordate"])
    if not success or not output:
        return None
    tags = output.split('\n')
    return tags[0] if tags else None


def get_commit_range(since: Optional[str] = None) -> List[Dict]:
    """
    获取指定范围的提交记录
    - since 为 None 时，从初始提交开始
    - 否则从 since..HEAD
    """
    if since is None:
        range_spec = "--root"
    else:
        range_spec = f"{since}..HEAD"

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
    # 去掉末尾可能多余的 \x01，然后按 \x01 分割
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
            'body': body.strip()  # 保留内部换行，去除首尾空白
        })

    return commits


def generate_markdown(commits: List[Dict], tag: Optional[str] = None) -> str:
    """生成 Markdown 内容"""
    time_full = datetime.now().strftime("%Y-%m-%d %H:%M:%S %Z")
    lines = []

    # 标题
    if tag is None:
        lines.append("# 变更日志 (Change Log) - 首次发布\n")
    else:
        lines.append("# 变更日志 (Change Log)\n")

    # 元信息
    lines.append(f"**生成时间**: {time_full} \n")

    if tag is None:
        lines.append("**版本范围**: 初始提交 → HEAD  \n")
    else:
        lines.append(f"**版本范围**: {tag} → HEAD  \n")

    lines.append(f"**提交总数**: {len(commits)}  \n")
    lines.append("\n---\n")

    if not commits:
        if tag:
            lines.append("\n## ✅ 无新提交\n")
            lines.append(f"\n最新标签 `{tag}` 已指向 HEAD，没有新的变更。\n")
        else:
            lines.append("\n## ⚠️ 没有提交记录\n")
            lines.append("\n仓库中没有任何提交。\n")
        return ''.join(lines)

    # 提交列表
        # 提交列表
    lines.append("\n## 📦 提交列表\n")

    digits = len(str(len(commits)))
    for idx, commit in enumerate(commits, start=1):
        seq = f"{idx:0{digits}d}"
        lines.append(f"\n### {seq}-{commit['subject']}\n")
        lines.append(f"> Hash: {commit['short_hash']}   At: {commit['full_time']}\n")
        if commit['body']:
            lines.append(commit['body'] + "\n")
        if idx != len(commit):
            lines.append("\n---\n")

    lines.append("结束")
    return ''.join(lines)


def save_changelog(content: str, tag: Optional[str] = None) -> Path:
    """保存 Change Log 到文件"""
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    if tag is None:
        filename = f"ChangesFirstUnused.md"
    else:
        safe_tag = tag.replace('/', '_').replace(' ', '_')
        filename = f"ChangesFrom{safe_tag}Unused.md"

    filepath = Path.cwd() / "docs/changes" / filename
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    return filepath


def main():
    # 检查是否在 Git 仓库中
    success, _ = run_git_command(["git", "rev-parse", "--git-dir"])
    if not success:
        print("❌ 错误：当前目录不是 Git 仓库")
        sys.exit(1)

    print("🔍 正在获取最新的标签...")
    latest_tag = get_latest_tag()

    if latest_tag is None:
        print("📌 未找到任何标签，生成首次发布的 Change Log...")
        commits = get_commit_range(since=None)
        tag = None
    else:
        print(f"📌 最新标签: {latest_tag}")
        print(f"🔍 检查 {latest_tag} 到 HEAD 之间的提交...")
        commits = get_commit_range(since=latest_tag)
        tag = latest_tag

    print(f"📊 共找到 {len(commits)} 个提交")

    print("📝 生成 Markdown 内容...")
    content = generate_markdown(commits, tag)

    filepath = save_changelog(content, tag)
    print(f"✅ Change Log 已保存到: {filepath}")


if __name__ == "__main__":
    main()
