from dataclasses import dataclass, field
from datetime import datetime

import unicodedata


def _display_width(s: str) -> int:
    """按等宽终端里的列数计算字符串宽度：CJK 全角字符算 2，其余算 1。"""
    return sum(
        2 if unicodedata.east_asian_width(ch) in ("W", "F") else 1
        for ch in s
    )


def _pad(s: str, width: int) -> str:
    """左对齐补空格到指定显示宽度。"""
    return s + " " * max(0, width - _display_width(s))


@dataclass
class CommitInfo:
    """一次 git 提交的摘要信息。

    派生字段由 __post_init__ 自动计算， 构造时不需要传入，也不应被外部覆盖。
    """
    commit_hash: str
    commit_time: datetime
    subject: str = ""
    body: str = ""
    author: str = "unknown"
    email: str = "unknown"
    abbreviated_hash: str = field(init=False)

    def __post_init__(self):
        if not self.commit_hash:
            raise ValueError("commit_hash must not be empty")
        self.abbreviated_hash = self.commit_hash[:7]

        self.commit_time = self.commit_time.astimezone()

    def to_markdown(self) -> str:
        left1 = f"Abbreviated hash: {self.abbreviated_hash}"
        left2 = f"Author: {self.author}"
        width = max(_display_width(left1), _display_width(left2))

        lines = [
            f"### {self.subject}",
            f"> {_pad(left1, width)}  |  At: {self.commit_time}  ",
            f"> {_pad(left2, width)}  |  Email: {self.email}",
            self.body.strip(),
        ]
        return "\n".join(lines)
