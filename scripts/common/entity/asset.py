from dataclasses import dataclass
from pathlib import Path

from common.entity.target import Target
from common.enums.kind import Kind


@dataclass(frozen=True, slots=True)
class Asset:
    """待发布产物：目标 + 载荷类型 + 本地路径"""

    target: Target
    kind: Kind
    path: Path
