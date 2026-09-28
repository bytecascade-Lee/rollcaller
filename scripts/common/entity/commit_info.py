from dataclasses import dataclass, field
from datetime import datetime

from util.str_format import display_width, padding


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
        width = max(display_width(left1), display_width(left2))

        lines = [
            f"### {self.subject}",
            f"> {padding(left1, width)}  |  At: {self.commit_time}  ",
            f"> {padding(left2, width)}  |  Email: {self.email}",
            self.body.strip(),
        ]
        return "\n".join(lines)
