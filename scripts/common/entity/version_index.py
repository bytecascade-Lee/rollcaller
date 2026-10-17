from collections.abc import Mapping
from dataclasses import dataclass, field

from common.entity.version import Version
from common.enums.severity import Severity


@dataclass(frozen=True, slots=True)
class VersionIndex:
    """版本索引条目：versions.json 数组中的一条记录"""

    version: Version
    severity: Severity = field(default_factory=Severity.default)

    @classmethod
    def from_dict(cls, data: Mapping[str, object]) -> "VersionIndex":
        """从 versions.json 的单条对象构造"""
        if not isinstance(data, Mapping):
            raise ValueError(f"版本索引条目不是对象: {data!r}")

        raw_version = data.get("version")
        if not isinstance(raw_version, str) or not raw_version.strip():
            raise ValueError("版本索引条目缺少合法的 version")
        try:
            version = Version.parse(raw_version)
        except ValueError as e:
            raise ValueError(f"version={raw_version!r} 非法: {e}") from e

        raw_severity = data.get("severity")
        if raw_severity is None:
            return cls(version=version)
        try:
            severity = Severity(raw_severity)
        except ValueError as e:
            legal = "/".join(repr(item.value) for item in Severity)
            raise ValueError(f"severity {raw_severity!r} 非法，应为 {legal}") from e
        return cls(version=version, severity=severity)

    def to_dict(self) -> dict[str, str]:
        """转为 versions.json 的单条对象"""
        return {"version": str(self.version), "severity": str(self.severity)}
