from common.entity.version import Version
from release import release_notes


def check_notes(version: Version, *, draft: bool) -> str:
    """草稿跳过发布说明；非草稿返回提取到的正文，失败抛出 release_notes 的领域异常"""
    if draft:
        return ""
    return release_notes.extract(version)
