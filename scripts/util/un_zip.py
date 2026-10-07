#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
压缩/解压缩工具
"""

import zipfile
from pathlib import Path
from typing import Iterable, List, Set, Union


def compress(
    base_dir: Path,
    targets: Union[Path, str, Iterable[Union[Path, str]]],
    output_path: Path,
    overwrite: bool = False,
) -> Path:
    """
    将 base_dir 下指定的文件/文件夹压缩为 zip

    :param base_dir:    源根目录
    :param targets:     相对于 base_dir 的路径或 glob 通配符，可以是单个路径/字符串或它们的可迭代对象
    :param output_path: 输出 zip 文件路径，包含文件名
    :param overwrite:   输出文件已存在时是否覆盖，默认 False，存在则抛 FileExistsError
    :return:            最终生成的 zip 文件路径
    """
    base_dir = Path(base_dir).resolve()
    output_path = Path(output_path).resolve()

    if not base_dir.is_dir():
        raise NotADirectoryError(f"base_dir {base_dir} 不是目录")

    if output_path.exists():
        if output_path.is_dir():
            raise IsADirectoryError(f"输出路径 {output_path} 不是文件")
        if not overwrite:
            raise FileExistsError(f"输出文件 {output_path} 已存在")

    # 统一 targets 为列表
    if isinstance(targets, (str, Path)):
        targets = [targets]
    else:
        targets = list(targets)

    matched: List[Path] = []
    for pattern in targets:
        pattern_str = str(pattern)
        if Path(pattern_str).is_absolute():
            raise ValueError(f"target 目录 {pattern_str} 必须是相对路径")

        # 特殊处理 "." 表示整个 base_dir
        if pattern_str in (".", "./"):
            matched.append(base_dir)
            continue

        hits = list(base_dir.glob(pattern_str))
        if not hits:
            raise FileNotFoundError(f"目标 {pattern_str} 不存在或无匹配")

        for hit in hits:
            # 确保路径在 base_dir 下
            try:
                hit.relative_to(base_dir)
            except ValueError:
                raise ValueError(f"目标超出 base_dir: {pattern_str} -> {hit}")
            matched.append(hit)

    # 去重并排序，保证输出顺序稳定
    matched = sorted(set(matched), key=lambda p: str(p))
    added: Set[str] = set()

    def add_path(path: Path, arcname: str) -> None:
        """递归添加路径到 zip，arcname 是 zip 内的相对路径"""
        if path == output_path:
            return

        if arcname == ".":
            arcname = ""
        # 规范化为 POSIX 风格，去掉开头的 /
        arcname = arcname.replace("\\", "/").lstrip("/")

        if path.is_dir():
            if arcname:
                dir_arc = arcname.rstrip("/") + "/"
                if dir_arc not in added:
                    added.add(dir_arc)
                    zf.write(path, dir_arc)
            for child in sorted(path.iterdir(), key=lambda p: p.name):
                child_arc = f"{arcname.rstrip('/')}/{child.name}" if arcname else child.name
                add_path(child, child_arc)
        else:
            if not arcname or arcname in added:
                return
            added.add(arcname)
            zf.write(path, arcname)

    with zipfile.ZipFile(output_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for path in matched:
            arcname = path.relative_to(base_dir).as_posix()
            add_path(path, arcname)

    return output_path
