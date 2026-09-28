import unicodedata


def display_width(s: str) -> int:
    """按等宽终端里的列数计算字符串宽度：CJK 全角字符算 2，其余算 1。"""
    return sum(
        2 if unicodedata.east_asian_width(ch) in ("W", "F") else 1
        for ch in s
    )


def padding(s: str, width: int) -> str:
    """左对齐补空格到指定显示宽度。"""
    return s + " " * max(0, width - display_width(s))
