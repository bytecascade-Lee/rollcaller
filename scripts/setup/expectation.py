#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
体检期望面
"""

from dataclasses import dataclass
from enum import StrEnum

from common.entity.version import Version

# 不符时的默认建议
ADVICE = "执行 `just setup`"


class Relation(StrEnum):
    """实际值与期望值之间要满足的关系"""

    # 相等
    EQ = "eq"
    # 不低于
    GE = "ge"
    # 存在（真值即可，不看 expected）
    EXISTS = "exists"


@dataclass(frozen=True, slots=True)
class Expectation[K]:
    """
    一条检查

    :param key: 域内标识，探针的入参
    :param label: 可读对象名，用于结论行与问题描述
    :param expected: 期望值，配合 relation 解释；EXISTS 不看它
    :param relation: 判定关系，默认相等
    :param advice: 不符时的建议，默认为 `ADVICE`
    """

    key: K
    label: str
    expected: object
    relation: Relation = Relation.EQ
    advice: str = ADVICE

    def judge(self, actual: object) -> str | None:
        """判定一条期望；一致返回 None，不一致返回一句带建议的描述"""
        match self.relation:
            case Relation.EQ:
                return self._judge_eq(actual)
            case Relation.GE:
                return self._judge_ge(actual)
            case Relation.EXISTS:
                return self._judge_exists(actual)
            case _ as unreachable:
                raise AssertionError(unreachable)

    def _judge_eq(self, actual: object) -> str | None:
        """相等判定；expected 为 None 时反向解释成「不该存在」"""
        if self.expected is None:
            return None if actual is None else self._sentence(f"应移除，当前 {actual!r}")
        if actual is None:
            return self._sentence(f"未设置，应为 {self.expected!r}")
        if actual != self.expected:
            return self._sentence(f"期望 {self.expected!r}，实际 {actual!r}")
        return None

    def _judge_ge(self, actual: object) -> str | None:
        """下限判定"""
        if not isinstance(actual, Version) or not isinstance(self.expected, Version):
            return self._sentence("的版本号无法识别")
        if actual < self.expected:
            return self._sentence(f"版本过低: {actual} < {self.expected}")
        return None

    def _judge_exists(self, actual: object) -> str | None:
        """存在性判定；actual 为真值即通过"""
        return None if actual else self._sentence("缺失")

    def _sentence(self, message: str) -> str:
        """把对象名、判定结论与建议拼成一句"""
        head = f"{self.label} {message}"
        return f"{head}；{self.advice}" if self.advice else head
