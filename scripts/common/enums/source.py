from enum import StrEnum


class Source(StrEnum):
    GITHUB = "github"
    CNB = "cnb"
    DEVELOP = "develop"

    @classmethod
    def values(cls) -> tuple[str, ...]:
        return tuple(source.value for source in cls)
