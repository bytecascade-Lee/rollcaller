import re

# Windows 文件名非法字符：< > : " / \ | ? * 以及 ASCII 控制字符 0x00-0x1F
# 额外把空白字符也一并替换，保持生成的文件名整洁
# （Windows 本身允许文件名中间含空格，此处仅为可读性考虑）
_WIN_ILLEGAL_CHARS = re.compile(r'[<>:"/\\|?*\x00-\x1f\s]+')

# Windows 保留设备名（不区分大小写，且带扩展名同样被保留，如 CON.txt）
_WIN_RESERVED_NAMES = frozenset(
    ["CON", "PRN", "AUX", "NUL"]
    + [f"COM{i}" for i in range(1, 10)]
    + [f"LPT{i}" for i in range(1, 10)]
)


def sanitize_filename(name: str, default: str = "unnamed") -> str:
    """
    将任意字符串清洗为可安全用作 Windows 文件名的形式

    处理内容：
        1. 非法字符 < > : " / \\ | ? * 及 ASCII 控制字符 -> 下划线，连续合并
        2. 空白字符 空格、Tab 等 -> 下划线
        3. 首尾的空格与点
        4. Windows 保留设备名 CON/PRN/AUX/NUL/COM1-9/LPT1-9 -> 前缀下划线
        5. 清洗后为空则返回 default

    未处理情况：
        1. 路径长度限制（MAX_PATH）
        2. Unicode 规范化
        3. 扩展名保留

    :param name: 待处理的文件名字
    :param default: 处理后名字若为空则返回的默认值
    """
    if not isinstance(name, str):
        name = str(name)

    cleaned = name.strip(" .")
    cleaned = _WIN_ILLEGAL_CHARS.sub("_", cleaned)

    if not cleaned:
        return default

    stem = cleaned.split(".", 1)[0].upper()
    if stem in _WIN_RESERVED_NAMES:
        cleaned = f"_{cleaned}"

    return cleaned
