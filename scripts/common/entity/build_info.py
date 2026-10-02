import json
from dataclasses import dataclass, field
from datetime import datetime
from typing import Optional


@dataclass
class BuildInfo:
    """一次构建的元数据。

    派生字段由 __post_init__ 自动计算，构造时不需要传入，也不应被外部覆盖。
    时间字段统一规范化为带系统时区的 datetime。
    """
    branch: Optional[str]
    commit_count: Optional[int]
    commit_hash: Optional[str]
    commit_time: Optional[datetime] = None
    build_time: Optional[datetime] = None
    abbreviated_hash: Optional[str] = field(init=False)

    def __post_init__(self):
        self.abbreviated_hash = self.commit_hash[:7] if self.commit_hash else None

        self.commit_time = self.commit_time.astimezone() if self.commit_time else None
        self.build_time = self.build_time.astimezone() if self.build_time else None

    def to_json(self) -> str:
        return json.dumps(
            self,
            default=lambda o: (
                o.isoformat(timespec="milliseconds")
                if isinstance(o, datetime)
                else o.__dict__
            ),
            sort_keys=False,
            indent=4,
        )
