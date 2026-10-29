from enum import StrEnum


class Profile(StrEnum):
    """Cargo 构建配置"""

    RELEASE = "release"
    DEBUG = "debug"
    DEVELOP = "develop"
