from dataclasses import dataclass, field

from common.enums.action import Action, UnsupportedActionError
from common.enums.arch import ArchTypeError, Arch
from common.enums.kind import Kind
from common.enums.os import OSTypeError, OS
from util.str_format import join_repr


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

    def __str__(self) -> str:
        return self.triple


@dataclass(frozen=True, slots=True)
class SupportedTarget:
    target: Target
    actions: frozenset[Action]
    kinds: frozenset[Kind]
    runner: str

    def supports(self, action: Action) -> bool:
        """是否开放指定动作档位；不做 BUILD / RELEASE 之间的隐含推导"""
        return action in self.actions

    def require(self, action: Action) -> None:
        """要求开放指定动作档位，否则抛 UnsupportedActionError"""
        if self.supports(action):
            return
        actions = join_repr(a.value for a in self.actions)
        raise UnsupportedActionError(f"目标 {self.target.triple!r} 未开放 {action.value!r} 档位，当前已开放 {actions}")
