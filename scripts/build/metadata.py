import os
from pathlib import Path

from cli import git


def ref_name(*, cwd: Path | None = None) -> str:
    github_ref_type = os.environ.get("GITHUB_REF_TYPE")
    current_branch = git.current_branch(cwd=cwd)
    github_ref_name = os.environ.get("GITHUB_REF_NAME")
    if github_ref_type == "tag" or not current_branch:
        return "detached"
    if github_ref_type == "branch" and github_ref_name:
        return github_ref_name
    return current_branch


def build_metadata(*, cwd: Path | None = None) -> str | None:
    """获取构建信息字符串：<ref_name>.<提交总数>.<短哈希>"""
    name = ref_name(cwd=cwd)
    count = git.commit_count(cwd=cwd)
    _hash = git.head_hash(cwd=cwd)[git.ABBREVIATED_HASH_SLICE]
    return f"{name}.{count}.{_hash}"
