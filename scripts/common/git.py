#!/usr/bin/env python3
"""
Git 操作模块：封装常用 git 命令，返回结构化数据

所有函数均无环境嗅探，失败时抛出异常，由调用方决定如何处理。
"""

import subprocess
from pathlib import Path
from typing import List, Optional, Tuple


class GitError(Exception):
    """Git 操作失败的基类"""
    pass


class NotInRepoError(GitError):
    """不在 Git 仓库中"""
    pass


def git(args: List[str], cwd: Optional[Path] = None) -> str:
    """
    执行 git 命令，返回 stdout 字符串

    Raises:
        NotInRepoError: 不在 git 仓库中
        GitError: 其他 git 错误
    """
    try:
        result = subprocess.run(
            ["git", *args],
            cwd=cwd,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
    except FileNotFoundError:
        raise GitError("未找到 git 命令，请确保已安装 Git")

    if result.returncode != 0:
        stderr = result.stderr.strip()
        if not is_repo(cwd):
            raise NotInRepoError(f"当前目录{cwd}不在 Git 仓库中")
        raise GitError(f"git {' '.join(args)} 失败: {stderr}")

    return result.stdout.strip()


def is_repo(cwd: Optional[Path] = None) -> bool:
    """检查当前目录是否在 Git 仓库中"""
    try:
        git(["rev-parse", "--git-dir"], cwd)
        return True
    except GitError:
        return False


def get_head_hash(short: bool = False, cwd: Optional[Path] = None) -> str:
    """
    获取 HEAD 的 commit hash

    Args:
        short: True 返回短 hash（7位），False 返回完整 hash
        cwd: 当前工作目录
    """
    if short:
        return git(["rev-parse", "--short", "HEAD"], cwd)
    return git(["rev-parse", "HEAD"], cwd)


def get_branch(cwd: Optional[Path] = None) -> str:
    """
    获取当前分支名

    如果处于 detached HEAD 状态，返回 "detached"
    """
    try:
        branch = git(["rev-parse", "--abbrev-ref", "HEAD"], cwd)
        if branch == "HEAD":
            # detached HEAD 状态，尝试获取 tag 或 fallback
            try:
                tag = git(["describe", "--tags", "--exact-match"], cwd)
                return f"detached@{tag}"
            except GitError:
                # 没有 tag 指向当前 commit
                short_hash = get_head_hash(True, cwd)
                return f"detached@{short_hash}"
        return branch
    except GitError:
        # 某些 edge case 下 rev-parse 返回 HEAD 但实际是 detached
        # 用 symbolic-ref 再确认一次
        try:
            branch = git(["symbolic-ref", "--short", "HEAD"], cwd)
            return branch
        except GitError:
            return "detached"


def get_commit_count(cwd: Optional[Path] = None) -> int:
    """获取当前分支的 commit 总数（从初始提交到 HEAD）"""
    output = git(["rev-list", "--count", "HEAD"], cwd)
    return int(output)


def get_latest_tag(cwd: Optional[Path] = None) -> Optional[str]:
    """
    获取最新的 tag（按创建时间排序）

    Returns:
        最新的 tag 名称，如果没有 tag 则返回 None
    """
    output = git(["tag", "--sort=-creatordate"], cwd)
    if not output:
        return None
    return output.split("\n")[0]


def get_tags_since(commit: str, cwd: Optional[Path] = None) -> List[str]:
    """
    获取从某个 commit 之后创建的所有 tag

    Args:
        commit: 起始 commit（不包含该 commit 本身）
        cwd: 当前工作目录

    Returns:
        tag 列表，按创建时间从新到旧排序
    """
    output = git(["tag", "--sort=-creatordate", f"--contains={commit}"], cwd)
    if not output:
        return []
    return output.split("\n")


def get_build_info(cwd: Optional[Path] = None) -> str:
    """
    获取构建信息字符串：分支名.提交总数.短哈希

    分支名中的 '/' 替换为 '-',避免文件路径问题

    Returns:
        格式: "<branch>.<count>.<short_hash>"
        例如: "master.142.a1b2c3d"

    Raises:
        NotInRepoError: 不在 git 仓库中
        GitError: git 命令执行失败
    """
    branch = get_branch(cwd).replace("/", "-")
    count = get_commit_count(cwd)
    short_hash = get_head_hash(True, cwd)
    return f"{branch}.{count}.{short_hash}"


def are_clean(files, cwd: Optional[Path] = None) -> Tuple[bool, str]:
    """
    判断给定文件在工作区是否无未提交改动（干净）。

    Args:
        files: 文件路径（Path 或 str）的可迭代对象
        cwd: 当前工作目录
    Returns:
        True 表示这些文件相对 HEAD 无改动
    """
    paths = [str(f) for f in files]
    output = git(["status", "--porcelain", "--", *paths], cwd=cwd)
    return output.strip() == "", output


def restore_files(files, cwd: Optional[Path] = None) -> None:
    """
    用 git checkout 还原给定文件到 HEAD 状态（丢弃工作区改动）。

    用于本地构建后还原 update_version 临时改动的版本号文件，避免脚本写文件
    引入换行符差异。

    Args:
        files: 文件路径（Path 或 str）的可迭代对象
        cwd: 当前工作目录
    """
    paths = [str(f) for f in files]
    git(["checkout", "--", *paths], cwd)


def get_commit_range(
    start: Optional[str] = None,
    end: Optional[str] = None,
    cwd: Optional[Path] = None,
) -> List[dict]:
    """
    获取指定范围的提交记录

    Args:
        start: 起始 commit/tag（不包含），None 表示从 root 开始
        end: 结束 commit/tag（包含），None 表示 HEAD
        cwd: 当前工作目录

    Returns:
        提交列表，每个提交包含以下字段：
            full_hash: 完整 hash
            short_hash: 短 hash
            timestamp: Unix 时间戳
            full_time: ISO 格式时间
            subject: 提交标题
            body: 提交正文
    """
    # 构建范围字符串
    if start is None:
        range_spec = "--root" if end is None else end
    else:
        range_spec = f"{start}..{end}" if end else f"{start}..HEAD"

    # 使用分隔符解析
    format_str = "%H%x00%h%x00%ct%x00%ci%x00%s%x00%b%x01"
    cmd = [
        "log",
        range_spec,
        f"--pretty=format:{format_str}",
        "--reverse",
    ]

    output = git(cmd, cwd=cwd)
    if not output:
        return []

    commits = []
    for block in output.rstrip("\x01").split("\x01"):
        if not block:
            continue
        parts = block.split("\x00")
        if len(parts) < 6:
            continue
        full_hash, short_hash, timestamp, full_time, subject, body = parts[:6]
        commits.append({
            "full_hash": full_hash,
            "short_hash": short_hash,
            "timestamp": int(timestamp),
            "full_time": full_time,
            "subject": subject.strip(),
            "body": body.strip(),
        })

    commits.sort(key=lambda x: x["timestamp"])
    return commits
