from enum import StrEnum

_ORDER = {
    "normal": 0,
    "important": 1,
    "critical": 2,
}


class Severity(StrEnum):
    NORMAL = "normal"
    IMPORTANT = "important"
    CRITICAL = "critical"

    @classmethod
    def default(cls) -> "Severity":
        return Severity.NORMAL

    @property
    def _rank(self) -> int:
        return _ORDER[self.value]

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
