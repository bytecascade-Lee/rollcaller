from enum import StrEnum


class UnsupportedActionError(Exception):
    """目标动作未开放到所需档位"""


class Action(StrEnum):
    """支持动作"""

    BUILD = "build"
    RELEASE = "release"
    TEST = "test"
