import unicodedata
from collections.abc import Iterable


def join_repr(items: Iterable[object], *, sep: str = "、") -> str:
    """把一组对象按 repr 拼成清单，供错误信息指出具体的名字。"""
    return sep.join(repr(item) for item in items)


def display_width(s: str) -> int:
    """按等宽终端里的列数计算字符串宽度：CJK 全角字符算 2，其余算 1。"""
    return sum(
        2 if unicodedata.east_asian_width(ch) in ("W", "F") else 1
        for ch in s
    )


def padding(s: str, width: int) -> str:
    """左对齐补空格到指定显示宽度。"""
    return s + " " * max(0, width - display_width(s))
