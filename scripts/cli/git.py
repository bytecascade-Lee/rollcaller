#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
Git 操作模块：封装常用 git 命令，返回结构化数据
"""

import os
import subprocess
from pathlib import Path
from typing import Iterable, Sequence

from cli import process
from cli.version import version_string
from common.constant.paths import PROJECT_BASE_DIR
from common.logger import debug

# 单条 git 命令的超时（秒）
DEFAULT_TIMEOUT = 60

ABBREVIATED_HASH_SLICE = slice(0, 10)


class GitError(Exception):
    """git 命令操作失败，未找到 git 命令，或命令超时，或命令执行失败"""


def _run(args: Sequence[str], *, cwd: Path | None = None, timeout: float | int = DEFAULT_TIMEOUT,
         allow: tuple[int, ...] = (0,), text: bool = True) -> subprocess.CompletedProcess:
    """
    执行 git 命令并返回 CompletedProcess

    统一加 `--no-pager` 与 `core.quotepath=false`，并固定 `LC_ALL=C`（输出可解析）、
    关闭终端交互提示（凭据缺失时快速失败而不是挂在子进程里）。

    :param args: git 子命令及其参数，如 ["rev-parse", "HEAD"]
    :param cwd: 工作目录，缺省 PROJECT_BASE_DIR
    :param timeout: 超时秒数
    :param allow: 额外允许的退出码，例如 `rev-parse --verify` 的 1 表示对象不存在
    :param text: True 返回 str，False 返回原始 bytes

    :raise GitError: 未找到 git 命令，或命令超时，或命令执行失败
    """
    try:
        return process.run(
            ["git", "--no-pager", "-c", "core.quotepath=false", *args],
            cwd=cwd or PROJECT_BASE_DIR,
            env={**os.environ, "LC_ALL": "C", "GIT_TERMINAL_PROMPT": "0"},
            timeout=timeout,
            text=text,
            allow=allow,
        )
    except process.ProcessError as error:
        raise GitError(str(error)) from error


def _text(result: subprocess.CompletedProcess) -> str:
    """CompletedProcess.stdout -> 字符串并 strip；非法字节已在 cli.process 中替换为 U+FFFD"""
    return str(result.stdout).strip()


def _paths(paths: Iterable[str | Path]) -> list[str]:
    """把 Path / str 混用的路径集合归一成 str 列表"""
    return [str(path) for path in paths]


def stdout(args: Sequence[str], *, cwd: Path | None = None, timeout: float | int = DEFAULT_TIMEOUT) -> str:
    """执行 git 命令，返回 utf-8 解码并 strip 后的 stdout"""
    return _text(_run(args, cwd=cwd, timeout=timeout))


def stdout_bytes(args: Sequence[str], *, cwd: Path | None = None, timeout: float | int = DEFAULT_TIMEOUT) -> bytes:
    """执行 git 命令，返回未解码的 stdout（blob 可能是二进制）"""
    return _run(args, cwd=cwd, timeout=timeout, text=False).stdout


def version() -> str | None:
    """`git --version` 里的版本号；输出认不出时返回 None"""
    return version_string(stdout(["--version"]))


def is_repo(cwd: Path | None = None) -> bool:
    """检查当前目录是否在 Git 仓库中"""
    result = _run(["rev-parse", "--git-dir"], cwd=cwd, allow=(128,))
    return result.returncode == 0


def rev_parse(ref: str, *, cwd: Path | None = None) -> str:
    """解析 ref 为完整对象哈希，不存在抛 GitError"""
    return stdout(["rev-parse", "--verify", ref], cwd=cwd)


def try_rev_parse(ref: str, *, cwd: Path | None = None) -> str | None:
    """同 rev_parse，但 ref 不存在时返回 None"""
    result = _run(["rev-parse", "--verify", "--quiet", ref], cwd=cwd, allow=(1,))
    if result.returncode != 0:
        return None
    return _text(result) or None


def head_hash(*, cwd: Path | None = None) -> str:
    """HEAD 的完整提交哈希"""
    return rev_parse("HEAD", cwd=cwd)


def current_branch(*, cwd: Path | None = None) -> str | None:
    """当前分支名；detached HEAD（含 tag 检出）返回 None"""
    result = _run(["symbolic-ref", "--quiet", "--short", "HEAD"], cwd=cwd, allow=(1,))
    if result.returncode != 0:
        return None
    return _text(result) or None


def commit_count(*, cwd: Path | None = None) -> int:
    """HEAD 的祖先提交总数"""
    output = stdout(["rev-list", "--count", "HEAD"], cwd=cwd)
    try:
        return int(output)
    except ValueError as error:
        raise GitError(f"git rev-list --count HEAD 输出非整数: {output!r}") from error


def commit_unix_time(ref: str = "HEAD", *, cwd: Path | None = None) -> int:
    """提交的 committer 时间；仓库无提交时 git 会以 128 退出并抛 GitError"""
    output = stdout(["log", "-1", "--format=%ct", ref], cwd=cwd)
    if not output:
        raise GitError(f"{ref} 没有 committer 时间，仓库可能尚无提交")
    try:
        return int(output.splitlines()[0])
    except ValueError as error:
        raise GitError(f"git log --format=%ct 输出非整数: {output!r}") from error


def remote_url(name: str = "origin", *, cwd: Path | None = None) -> str | None:
    """远端 URL；该远端不存在返回 None"""
    # git 用退出码 2 表示 no such remote
    result = _run(["remote", "get-url", name], cwd=cwd, allow=(2, 128))
    if result.returncode != 0:
        return None
    return _text(result) or None


def set_config(key: str, value: str, *, cwd: Path | None = None) -> None:
    """
    写入仓库级配置

    :param key: 配置键，如 `pull.ff`
    :param value: 配置值，如 `only`
    :param cwd: 当前工作目录
    """
    stdout(["config", "--local", key, value], cwd=cwd)


def get_config(key: str, *, cwd: Path | None = None) -> str | None:
    """读取仓库级配置，未设置返回 None"""
    result = _run(["config", "--local", "--get", key], cwd=cwd, allow=(1,))
    if result.returncode != 0:
        return None
    return _text(result) or None


def tags(pattern: str = "*", *, cwd: Path | None = None) -> list[str]:
    """列出匹配 pattern 的 tag 名；`*` 由 git 解析，不经过 shell"""
    output = stdout(["tag", "--list", pattern], cwd=cwd)
    return [line for line in output.splitlines() if line]


def create_tag(tag: str, message: str, *, cwd: Path | None = None) -> None:
    """在 HEAD 上创建附注 tag（`-a -m`）"""
    stdout(["tag", "-a", tag, "-m", message], cwd=cwd)


def rename_remote(old: str, new: str, *, cwd: Path | None = None) -> None:
    """重命名远端；git 会一并改写 `branch.<name>.remote` 与远程跟踪引用"""
    stdout(["remote", "rename", old, new], cwd=cwd)


def remotes(*, cwd: Path | None = None) -> list[str]:
    """远端名列表；克隆来的仓库通常只有一个 `origin`"""
    output = stdout(["remote"], cwd=cwd)
    return [line for line in output.splitlines() if line]


def tracked_changes(*, cwd: Path | None = None) -> str:
    """已跟踪文件的改动（porcelain，忽略未跟踪文件）；空串表示工作区干净"""
    return stdout(["status", "--porcelain", "--untracked-files=no"], cwd=cwd)


def status_porcelain(paths: Iterable[str | Path], *, cwd: Path | None = None) -> str:
    """执行 git status --porcelain -- <paths>；空串表示这些路径相对 HEAD 无改动"""
    return stdout(["status", "--porcelain", "--", *_paths(paths)], cwd=cwd)


def checkout_paths(paths: Iterable[str | Path], *, cwd: Path | None = None) -> None:
    """丢弃工作区给定路径的改动"""
    stdout(["checkout", "--", *_paths(paths)], cwd=cwd)


def show_blob(ref: str, path: str, *, cwd: Path | None = None) -> bytes | None:
    """取 `<ref>:<path>` 的 blob 原始内容，该路径在 ref 下不存在时返回 None"""
    try:
        return stdout_bytes(["show", f"{ref}:{path}"], cwd=cwd)
    except GitError as error:
        debug(f"{ref}:{path} 读取失败，按「不存在」处理: {error}")
        return None
