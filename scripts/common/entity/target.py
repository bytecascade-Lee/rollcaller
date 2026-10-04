from dataclasses import dataclass, field

from common.enums.arch import ArchTypeError, Arch
from common.enums.os import OSTypeError, OS


class UnsupportedTargetError(Exception):
    """不支持的 rust 目标三元组"""
    pass


@dataclass(frozen=True, slots=True)
class Target:
    """构建目标"""

    triple: str
    os: OS = field(init=False)
    arch: Arch = field(init=False)

    def __post_init__(self) -> None:
        triple = self.triple.strip().lower()
        object.__setattr__(self, "triple", triple)
        object.__setattr__(self, "os", self.parse_os(triple))
        object.__setattr__(self, "arch", self.parse_arch(triple))

    @staticmethod
    def parse_os(triple: str) -> OS:
        for token in triple.split("-"):
            found = OS.parse(token)
            if found is not None:
                return found
        raise OSTypeError(f"无法从目标三元组 {triple!r} 识别出操作系统")

    @staticmethod
    def parse_arch(triple: str) -> Arch:
        token = triple.split("-", 1)[0]
        arch = Arch.parse(token)
        if arch is None:
            raise ArchTypeError(f"无法从目标三元组 {triple!r} 识别出 CPU 架构")
        return arch
