from enum import StrEnum
from typing import assert_never


class Severity(StrEnum):
    NORMAL = "normal"
    IMPORTANT = "important"
    CRITICAL = "critical"

    @classmethod
    def default(cls) -> "Severity":
        return Severity.NORMAL

    @classmethod
    def values(cls) -> tuple[str]:
        return tuple(severity.value for severity in cls)

    @property
    def _rank(self) -> int:
        match self:
            case Severity.NORMAL:
                return 0
            case Severity.IMPORTANT:
                return 1
            case Severity.CRITICAL:
                return 2
            case _:
                assert_never(self)

    def __lt__(self, other):
        if isinstance(other, Severity):
            return self._rank < other._rank
        return NotImplemented

    def __le__(self, other):
        if isinstance(other, Severity):
            return self._rank <= other._rank
        return NotImplemented

    def __gt__(self, other):
        if isinstance(other, Severity):
            return self._rank > other._rank
        return NotImplemented

    def __ge__(self, other):
        if isinstance(other, Severity):
            return self._rank >= other._rank
        return NotImplemented
