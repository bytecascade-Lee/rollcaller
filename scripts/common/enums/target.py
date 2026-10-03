from dataclasses import dataclass, field
from enum import StrEnum


class OSTypeError(Exception):
    """os 无法解析：未识别的操作系统类型"""
    pass


class ArchTypeError(Exception):
    """arch 无法解析：未识别的CPU 架构类型"""
    pass


class UnsupportedTargetError(Exception):
    """不支持的 rust 目标三元组"""
    pass


class OS(StrEnum):
    """操作系统"""

    WINDOWS = "windows"
    MACOS = "macos"
    LINUX = "linux"

    @classmethod
    def parse(cls, raw: str) -> "OS | None":
        token = raw.strip().lower()
        if token == "windows":
            return cls.WINDOWS
        if token in {"macos", "darwin"}:
            return cls.MACOS
        if token == "linux":
            return cls.LINUX
        return None


class Arch(StrEnum):
    """CPU 架构"""

    X86_64 = "x86_64"
    ARM64 = "arm64"

    @classmethod
    def parse(cls, raw: str) -> "Arch | None":
        token = raw.strip().lower()
        if token in {"x86_64", "x64", "amd64"}:
            return cls.X86_64
        if token in {"arm64", "aarch64"}:
            return cls.ARM64
        return None


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
