from enum import StrEnum


class Mode(StrEnum):
    LOCAL = "local"
    CI = "ci"

    @classmethod
    def values(cls) -> tuple[str]:
        return tuple(mode.value for mode in cls)
