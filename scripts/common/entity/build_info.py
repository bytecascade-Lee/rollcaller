import json
from dataclasses import dataclass, field
from datetime import datetime


@dataclass
class BuildInfo:
    """一次构建的元数据。

    派生字段由 __post_init__ 自动计算，构造时不需要传入，也不应被外部覆盖。
    时间字段统一规范化为带系统时区的 datetime。
    """
    branch: str
    commit_count: int
    commit_hash: str
    commit_time: datetime
    build_time: datetime
    abbreviated_hash: str = field(init=False)

    def __post_init__(self):
        if not self.commit_hash:
            raise ValueError("commit_hash must not be empty")
        self.abbreviated_hash = self.commit_hash[:7]

        self.commit_time = self.commit_time.astimezone()
        self.build_time = self.build_time.astimezone()

    def to_json(self) -> str:
        return json.dumps(
            self,
            default=self._json_default,
            sort_keys=True,
            indent=4,
        )

    @staticmethod
    def _json_default(o):
        if isinstance(o, datetime):
            return o.isoformat(timespec="milliseconds")
        if hasattr(o, "__dict__"):
            return o.__dict__
        raise TypeError(f"Object of type {type(o).__name__} is not JSON serializable")
