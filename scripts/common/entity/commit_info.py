from dataclasses import dataclass, field
from datetime import datetime


@dataclass
class CommitInfo:
    """一次 git 提交的摘要信息"""
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
        self.abbreviated_hash = self.commit_hash[:10]
        self.commit_time = self.commit_time.astimezone()
