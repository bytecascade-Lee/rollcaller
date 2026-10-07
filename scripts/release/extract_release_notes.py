import re

from common.constant.paths import RELEASE_NOTES_PATH
from common.entity.version import Version


class ReleaseNotesNotFound(Exception):
    pass


class ReleaseNotesAmbiguous(Exception):
    pass


class ReleaseNotesSectionEmpty(Exception):
    pass


def extract_release_notes(version: Version) -> str:
    """从 RELEASE_NOTES.md 提取 '## <version> {}' 章节"""
    if not RELEASE_NOTES_PATH.exists():
        raise ReleaseNotesNotFound(f"版本发布说明文件 {RELEASE_NOTES_PATH!r} 不存在")
    lines = RELEASE_NOTES_PATH.read_text(encoding="utf-8").splitlines()
    # 标题行允许带锚点后缀（`## 0.1.0 {#v0-1-0}`）
    pattern = rf"^## {version}(?:\s.*)?$"
    matches = [i for i, line in enumerate(lines) if re.match(pattern, line)]
    length = len(matches)
    if length > 1:
        detail = "、".join(f"第 {idx + 1} 行 {lines[idx]!r}" for idx in matches)
        raise ReleaseNotesAmbiguous(f"在 {str(RELEASE_NOTES_PATH)!r} 中找到 {length} 个匹配 {pattern!r} 的标题：{detail}")
    if not matches:
        raise ReleaseNotesSectionEmpty(f"在 {str(RELEASE_NOTES_PATH)!r} 中未找到匹配 {pattern!r} 的标题")
    body = []
    for line in lines[matches[0] + 1:]:
        if re.match(r"^##\s", line):
            break
        body.append(line)
    # 章节末尾与下一章之间的分隔线 `---` 不属于本章内容：
    text = re.sub(r"\n*-{3,}\s*$", "", "\n".join(body).rstrip()).rstrip()
    if not text:
        raise ReleaseNotesSectionEmpty(f"在 {str(RELEASE_NOTES_PATH)!r} 中唯一匹配的 {pattern!r} 标题的章节内容为空")
    return text
