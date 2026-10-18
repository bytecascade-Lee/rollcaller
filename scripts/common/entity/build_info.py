from dataclasses import dataclass, field
from datetime import datetime


@dataclass
class BuildInfo:
    """一次构建的元数据，即 build_info.json 的全部内容"""

    ref_name: str
    commit_count: int
    commit_hash: str
    commit_time: datetime
    build_time: datetime
    abbreviated_hash: str = field(init=False)

    def __post_init__(self):
        self.abbreviated_hash = self.commit_hash[:10]

        self.commit_time = self.commit_time.astimezone()
        self.build_time = self.build_time.astimezone()
