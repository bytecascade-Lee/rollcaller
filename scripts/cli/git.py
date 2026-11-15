#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
Git 操作模块：封装常用 git 命令，返回结构化数据
"""

import os
from collections.abc import Iterable, Mapping
from pathlib import Path

from cli._tool import Timeout, Tool
from common.constant.paths import PROJECT_BASE_DIR
from common.logger import debug

ABBREVIATED_HASH_SLICE = slice(0, 10)


class GitError(Exception):
    """git 命令操作失败，未找到 git 命令，或命令超时，或命令执行失败"""


# 统一加 `--no-pager` 与 `core.quotepath=false`，并固定 `LC_ALL=C`（输出可解析）、
# 关闭终端交互提示（凭据缺失时快速失败而不是挂在子进程里）。
TOOL = Tool(
    "git",
    GitError,
    cwd=PROJECT_BASE_DIR,
    timeout=Timeout.NORMAL,
    prefix=("--no-pager", "-c", "core.quotepath=false"),
    env={"LC_ALL": "C", "GIT_TERMINAL_PROMPT": "0"},
    hint="请安装 Git 并加入 PATH：https://git-scm.com/downloads",
)


def _paths(paths: Iterable[str | Path]) -> list[str]:
    """把 Path / str 混用的路径集合归一成 str 列表"""
    return [str(path) for path in paths]


def is_repo(cwd: Path | None = None) -> bool:
    """检查当前目录是否在 Git 仓库中"""
    result = TOOL.run(["rev-parse", "--git-dir"], cwd=cwd, timeout=Timeout.QUERY, allow=(128,))
    return result.returncode == 0


def rev_parse(ref: str, *, cwd: Path | None = None) -> str:
    """解析 ref 为完整对象哈希，不存在抛 GitError"""
    args = ["rev-parse", "--verify", ref]
    return TOOL.text(args, cwd=cwd)


def try_rev_parse(ref: str, *, cwd: Path | None = None) -> str | None:
    """同 rev_parse，但 ref 不存在时返回 None"""
    args = ["rev-parse", "--verify", "--quiet", ref]
    result = TOOL.run(args, cwd=cwd, allow=(1,))
    if result.returncode != 0:
        return None
    return str(result.stdout).strip() or None


def head_hash(*, cwd: Path | None = None) -> str:
    """HEAD 的完整提交哈希"""
    return rev_parse("HEAD", cwd=cwd)


def current_branch(*, cwd: Path | None = None) -> str | None:
    """当前分支名；detached HEAD（含 tag 检出）返回 None"""
    args = ["symbolic-ref", "--quiet", "--short", "HEAD"]
    result = TOOL.run(args, cwd=cwd, timeout=Timeout.QUERY, allow=(1,))
    if result.returncode != 0:
        return None
    return str(result.stdout).strip() or None


def commit_count(*, cwd: Path | None = None) -> int:
    """HEAD 的祖先提交总数"""
    args = ["rev-list", "--count", "HEAD"]
    output = TOOL.text(args, cwd=cwd, timeout=Timeout.QUERY)
    try:
        return int(output)
    except ValueError as error:
        raise GitError(f"git rev-list --count HEAD 输出非整数: {output!r}") from error


def commit_unix_time(ref: str = "HEAD", *, cwd: Path | None = None) -> int:
    """提交的 committer 时间；仓库无提交时 git 会以 128 退出并抛 GitError"""
    args = ["log", "-1", "--format=%ct", ref]
    output = TOOL.text(args, cwd=cwd, timeout=Timeout.QUERY)
    if not output:
        raise GitError(f"{ref} 没有 committer 时间，仓库可能尚无提交")
    try:
        return int(output.splitlines()[0])
    except ValueError as error:
        raise GitError(f"git log --format=%ct 输出非整数: {output!r}") from error


def remote_url(name: str = "origin", *, cwd: Path | None = None) -> str | None:
    """远端 URL；该远端不存在返回 None"""
    # git 用退出码 2 表示 no such remote
    args = ["remote", "get-url", name]
    result = TOOL.run(args, cwd=cwd, timeout=Timeout.QUERY, allow=(2, 128))
    if result.returncode != 0:
        return None
    return str(result.stdout).strip() or None


def set_config(key: str, value: str, *, cwd: Path | None = None) -> None:
    """写入仓库级配置"""
    args = ["config", "--local", key, value]
    _ = TOOL.text(args, cwd=cwd)


def unset_config(key: str, *, cwd: Path | None = None) -> None:
    """删除仓库级配置；该键本来就不存在时也算成功（`git config --unset` 以 5 退出）"""
    args = ["config", "--local", "--unset", key]
    _ = TOOL.run(args, cwd=cwd, allow=(5,))


def get_config(key: str, *, cwd: Path | None = None) -> str | None:
    """读取仓库级配置，未设置返回 None"""
    args = ["config", "--local", "--get", key]
    result = TOOL.run(args, cwd=cwd, timeout=Timeout.QUERY, allow=(1,))
    if result.returncode != 0:
        return None
    return str(result.stdout).strip() or None


def tags(pattern: str = "*", *, cwd: Path | None = None) -> list[str]:
    """列出匹配 pattern 的 tag 名；`*` 由 git 解析，不经过 shell"""
    args = ["tag", "--list", pattern]
    output = TOOL.text(args, cwd=cwd)
    return [line for line in output.splitlines() if line]


def create_tag(tag: str, message: str, *, cwd: Path | None = None) -> None:
    """在 HEAD 上创建附注 tag（`-a -m`）"""
    args = ["tag", "-a", tag, "-m", message]
    _ = TOOL.text(args, cwd=cwd)


def rename_remote(old: str, new: str, *, cwd: Path | None = None) -> None:
    """重命名远端；git 会一并改写 `branch.<name>.remote` 与远程跟踪引用"""
    args = ["remote", "rename", old, new]
    _ = TOOL.text(args, cwd=cwd)


def add_remote(name: str, url: str, *, cwd: Path | None = None) -> None:
    """新增远端；不 fetch，跟踪引用等第一次 `git fetch <name>` 才建立"""
    args = ["remote", "add", name, url]
    _ = TOOL.text(args, cwd=cwd)


def remotes(*, cwd: Path | None = None) -> list[str]:
    """远端名列表；克隆来的仓库通常只有一个 `origin`"""
    args = ["remote"]
    output = TOOL.text(args, cwd=cwd, timeout=Timeout.QUERY)
    return [line for line in output.splitlines() if line]


def tracked_changes(*, cwd: Path | None = None) -> str:
    """已跟踪文件的改动（porcelain，忽略未跟踪文件）；空串表示工作区干净"""
    args = ["status", "--porcelain", "--untracked-files=no"]
    return TOOL.text(args, cwd=cwd)


def status_porcelain(paths: Iterable[str | Path], *, cwd: Path | None = None) -> str:
    """执行 git status --porcelain -- <paths>；空串表示这些路径相对 HEAD 无改动"""
    args = ["status", "--porcelain", "--", *_paths(paths)]
    return TOOL.text(args, cwd=cwd)


def diff_names(paths: Iterable[str | Path], *, against: str = "HEAD", cwd: Path | None = None) -> list[str]:
    """相对 against 有改动的路径清单，空列表表示无差异"""
    args = ["diff", "--name-only", "--exit-code", against, "--", *_paths(paths)]
    result = TOOL.run(args, cwd=cwd, allow=(0, 1))
    return [line for line in str(result.stdout).strip().splitlines() if line.strip()]


def untracked_names(paths: Iterable[str | Path], *, cwd: Path | None = None) -> list[str]:
    """未纳入版本控制的文件清单，空列表表示没有"""
    args = ["ls-files", "--others", "--exclude-standard", "--", *_paths(paths)]
    output = TOOL.text(args, cwd=cwd)
    return [line for line in output.splitlines() if line.strip()]


def checkout_paths(paths: Iterable[str | Path], *, cwd: Path | None = None) -> None:
    """丢弃工作区给定路径的改动"""
    args = ["checkout", "--", *_paths(paths)]
    _ = TOOL.text(args, cwd=cwd)


def checkout_branch(
    branch: str, *, start_point: str | None = None, force: bool = False, cwd: Path | None = None
) -> None:
    """
    切换分支

    :param branch: 分支名
    :param start_point: 起点（缺省为当前 HEAD）
    :param force: 用 `-B`；同名分支已存在时重置到 start_point，供流水线重跑覆盖机器人分支
    """
    args = ["checkout", "-B" if force else "-b", branch]
    if start_point is not None:
        args.append(start_point)
    _ = TOOL.text(args, cwd=cwd)


def add(paths: Iterable[str | Path], *, cwd: Path | None = None) -> None:
    """暂存指定路径"""
    args = ["add", "--", *_paths(paths)]
    _ = TOOL.text(args, cwd=cwd)


def commit(message: str, *, env: Mapping[str, str] | None = None, cwd: Path | None = None) -> None:
    """提交已暂存内容

    `env` 是**追加**的提交身份（`GIT_AUTHOR_*` / `GIT_COMMITTER_*`）：在当前环境上叠加，
    `env` 是替换语义，直接透传会让 git 丢掉 HOME 里的用户配置。
    """
    args = ["commit", "-m", message]
    _ = TOOL.run(args, cwd=cwd, env={**os.environ, **env} if env else None)


def push_branch(branch: str, *, remote: str = "origin", force: bool = False, cwd: Path | None = None) -> None:
    """推送分支到远端"""
    args = ["push", *(["--force"] if force else []), remote, f"refs/heads/{branch}:refs/heads/{branch}"]
    _ = TOOL.text(args, cwd=cwd, timeout=Timeout.NETWORK)


def show_blob(ref: str, path: str, *, cwd: Path | None = None) -> bytes | None:
    """取 `<ref>:<path>` 的 blob 原始内容，该路径在 ref 下不存在时返回 None"""
    try:
        args = ["show", f"{ref}:{path}"]
        return TOOL.bytes(args, cwd=cwd)
    except GitError as error:
        debug(f"{ref}:{path} 读取失败，按「不存在」处理: {error}")
        return None
