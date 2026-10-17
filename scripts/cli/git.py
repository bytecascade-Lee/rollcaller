#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
Git 操作模块：封装常用 git 命令，返回结构化数据

进程与错误映射统一在 _run() 中：
    任何失败均抛 GitError，调用方决定如何处理；
    空值返回 None，调用方需自行判空。

stdout 两种读法：
    git()             元数据 —— UTF-8 解码，非法字节替换为 U+FFFD 后 strip
    _capture_bytes()  内容   —— 原始字节
"""
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import List, Optional, Tuple

from common.constant.paths import PROJECT_BASE_DIR
from common.constant.repo import RELEASE_LINE
from common.entity.commit_info import CommitInfo
from common.logger import debug


class GitError(Exception):
    """Git 操作失败的基类"""
    pass


def _run(args: List[str], cwd: Optional[Path] = None) -> subprocess.CompletedProcess:
    """执行 git 命令，返回 CompletedProcess"""
    debug(f"执行: git {' '.join(args)}")
    try:
        result = subprocess.run(
            ["git", *args],
            cwd=cwd,
            capture_output=True,
            check=False,
        )
    except FileNotFoundError:
        raise GitError("未找到 git 命令，请确保已安装 Git")

    if result.returncode != 0:
        stderr = result.stderr.decode("utf-8", "replace").strip()
        raise GitError(f"git {' '.join(args)} 失败: {stderr}")

    return result


def git(args: List[str], cwd: Optional[Path] = None) -> str:
    """执行 git 命令，返回 stdout 字符串"""
    return _run(args, cwd).stdout.decode("utf-8", "replace").strip()


def _capture_bytes(args: List[str], cwd: Optional[Path] = None) -> bytes:
    """执行 git 命令，返回未经加工的 stdout 字节"""
    return _run(args, cwd).stdout


def get_blob(ref: str, path: str, cwd: Optional[Path] = None) -> Optional[bytes]:
    """取 `<ref>:<path>` 的 blob 原始内容"""
    try:
        return _capture_bytes(["show", f"{ref}:{path}"], cwd)
    except GitError as e:
        debug(f"{ref}:{path} 取不到内容，按「不存在」处理（{e}）")
        return None


def is_repo(cwd: Optional[Path] = None) -> bool:
    """检查当前目录是否在 Git 仓库中"""
    try:
        _run(["rev-parse", "--git-dir"], cwd)
        return True
    except GitError:
        return False


def get_head_hash(short: bool = False, cwd: Optional[Path] = None) -> str:
    """获取 HEAD 的 commit hash"""
    if short:
        return git(["rev-parse", "--short", "HEAD"], cwd)
    return git(["rev-parse", "HEAD"], cwd)


def get_branch(cwd: Optional[Path] = None) -> str:
    """获取当前分支名，如果处于 detached HEAD 状态，返回 "detached"""
    try:
        return git(["symbolic-ref", "--short", "HEAD"], cwd)
    except GitError:
        try:
            tag = git(["describe", "--tags", "--exact-match"], cwd)
            return f"detached@{tag}"
        except GitError:
            try:
                short_hash = get_head_hash(True, cwd)
                return f"detached@{short_hash}"
            except GitError:
                return "detached"


def source_branch() -> str:
    """
    来源分支：
        tag 发布  -> 发布线 RELEASE_LINE
        手动发布  -> 所选 ref
        本地/其它 -> 当前分支，detached HEAD 时形如 detached@<tag|短哈希>
    """
    ref_type = os.environ.get("GITHUB_REF_TYPE")
    if ref_type == "tag":
        return RELEASE_LINE
    ref_name = os.environ.get("GITHUB_REF_NAME")
    if ref_type == "branch" and ref_name:
        return ref_name
    return get_branch(PROJECT_BASE_DIR)


def get_commit_count(cwd: Optional[Path] = None) -> int:
    """获取当前分支的 commit 总数（从初始提交到 HEAD）"""
    output = git(["rev-list", "--count", "HEAD"], cwd)
    return int(output)


def get_description(dirty: bool = True, cwd: Optional[Path] = None) -> Optional[str]:
    try:
        args = ["describe", "--tags"]
        if dirty:
            args.append("--dirty")
        return git(args, cwd)
    except GitError:
        return None


def get_latest_tag(cwd: Optional[Path] = None) -> Optional[str]:
    """获取最新的 tag，按创建时间排序"""
    output = git(["tag", "--sort=-creatordate"], cwd)
    if not output:
        return None
    return output.split("\n")[0]


def get_last_commit_time(cwd: Optional[Path] = None) -> Optional[str]:
    output = git(["log", "-1", "--format=%ct"], cwd)
    if not output:
        return None
    return output.split("\n")[0]


def commit_exists(tag: str) -> bool:
    """检查提交是否存在"""
    try:
        git(["rev-parse", tag])
        return True
    except GitError:
        return False


def get_tags_since(commit: str, cwd: Optional[Path] = None) -> List[str]:
    """获取从某个 commit 之后创建的所有 tag"""
    output = git(["tag", "--sort=-creatordate", f"--contains={commit}"], cwd)
    if not output:
        return []
    return output.split("\n")


def get_build_info(cwd: Optional[Path] = None) -> str:
    """获取构建信息字符串：分支名.提交总数.短哈希"""
    branch = get_branch(cwd)
    count = get_commit_count(cwd)
    abbreviated_hash = get_head_hash(cwd=cwd)[:10]
    return f"{branch}.{count}.{abbreviated_hash}"


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
    start: str,
    end: str,
    cwd: Optional[Path] = None,
) -> List[CommitInfo]:
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

    output = git(cmd, cwd=cwd)
    if not output:
        return []

    commits: List[CommitInfo] = []
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
