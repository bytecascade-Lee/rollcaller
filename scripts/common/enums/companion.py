from enum import Enum


class Companion(Enum):
    SIGNATURE = ".sig"
    SHA256 = ".sha256"

    def __init__(self, suffix: str):
        self.suffix = suffix
