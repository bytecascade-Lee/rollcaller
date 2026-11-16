import os

from cli import git


def ref_name() -> str:
    github_ref_type = os.environ.get("GITHUB_REF_TYPE")
    current_branch = git.current_branch()
    github_ref_name = os.environ.get("GITHUB_REF_NAME")
    if github_ref_type == "tag" or not current_branch:
        return "detached"
    if github_ref_type == "branch" and github_ref_name:
        return github_ref_name
    return current_branch


def build_metadata() -> str | None:
    """获取构建信息字符串：<ref_name>.<提交总数>.<短哈希>"""
    name = ref_name()
    count = git.commit_count()
    _hash = git.head_hash()[git.ABBREVIATED_HASH_SLICE]
    return f"{name}.{count}.{_hash}"
