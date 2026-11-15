#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
外部工具的命令面
"""

import os
import re
import subprocess
from collections.abc import Callable, Collection, Mapping, Sequence
from dataclasses import dataclass
from enum import IntEnum
from pathlib import Path
from typing import Any

from cli import _process


class Timeout(IntEnum):
    """外部命令的超时档位（秒）"""

    # 瞬时查询
    QUERY = 30
    # 本地常规操作
    NORMAL = 60
    # 网络往返
    NETWORK = 180
    # 长任务
    EXTENDED = 1800


TimeoutOverride = Timeout | float | int | None

_VERSION_RE = re.compile(r"(\d+(?:\.\d+){0,2})")

# 把底层失败翻译成领域异常；返回 None 表示使用默认处理
Translator = Callable[[Sequence[str], _process.ProcessError], Exception | None]


def _merged_env(forced: Mapping[str, str] | None, override: Mapping[str, str] | None) -> Mapping[str, str] | None:
    """组合本次调用的环境变量"""
    if override is None and not forced:
        return None
    base = dict(os.environ) if override is None else dict(override)
    if forced:
        base.update(forced)
    return base


@dataclass(frozen=True, slots=True)
class Tool:
    """
    一个外部工具的命令面

    :param program: 可执行文件名，如 "git"
    :param error: 本工具的错误类型，构造签名是 (message)
    :param hint: 找不到可执行文件时的可操作提示，如安装方式或链接；空串则用 ProcessError 原文
    :param cwd: 缺省工作目录；None 表示继承调用方
    :param timeout: 缺省超时档位
    :param capture: 缺省是否捕获输出；长任务设 False 让输出直接进控制台
    :param search_dir: 只在指定目录里找可执行文件
    :param prefix: 每次调用都排在参数前面的固定 argv
    :param env: 每次调用都要叠加的固定环境变量
    :param translate: 领域错误翻译钩子，尽量别用
    """

    program: str
    error: type[Exception]
    hint: str = ""
    cwd: Path | None = None
    timeout: Timeout = Timeout.NORMAL
    capture: bool = True
    search_dir: Path | None = None
    prefix: tuple[str, ...] = ()
    env: Mapping[str, str] | None = None
    translate: Translator | None = None

    def run(
        self,
        args: Sequence[str],
        *,
        cwd: Path | None = None,
        timeout: TimeoutOverride = None,
        capture: bool | None = None,
        allow: Collection[int] = (),
        env: Mapping[str, str] | None = None,
        text: bool = True,
    ) -> subprocess.CompletedProcess[Any]:
        """
        执行一条命令

        :param args: 子命令与参数，如 ["fetch", "--locked"]
        :param cwd: 工作目录
        :param timeout: 覆盖描述符里的档位；None 表示沿用描述符默认值
        :param capture: 覆盖描述符里的捕获策略；None 表示沿用描述符
        :param allow: 额外允许的退出码，0 恒允许
        :param env: 本次调用的环境变量，**替换**语义；描述符里的 env 仍会叠加其上
        :param text: True 按 utf-8 解码，False 返回原始字节
        """
        try:
            return _process.run(
                [self.program, *self.prefix, *args],
                cwd=cwd or self.cwd,
                timeout=self.timeout if timeout is None else timeout,
                env=_merged_env(self.env, env),
                capture=self.capture if capture is None else capture,
                allow=allow,
                text=text,
                search_dir=self.search_dir,
            )
        except _process.ProcessError as failure:
            raise self._to_error(args, failure) from failure

    def text(
        self,
        args: Sequence[str],
        *,
        cwd: Path | None = None,
        timeout: TimeoutOverride = None,
        allow: Collection[int] = (),
        env: Mapping[str, str] | None = None,
    ) -> str:
        """执行命令并返回 strip 后的 stdout"""
        result = self.run(args, cwd=cwd, timeout=timeout, allow=allow, env=env, capture=True)
        return str(result.stdout).strip()

    def bytes(
        self,
        args: Sequence[str],
        *,
        cwd: Path | None = None,
        timeout: TimeoutOverride = None,
        allow: Collection[int] = (),
        env: Mapping[str, str] | None = None,
    ) -> bytes:
        """执行命令并返回未解码的 stdout"""
        return self.run(args, cwd=cwd, timeout=timeout, allow=allow, env=env, capture=True, text=False).stdout

    def version(self) -> str | None:
        """返回 `--version` 中的版本号；无法从输出中提取时返回 None"""
        match = _VERSION_RE.search(self.text(["--version"], timeout=Timeout.QUERY))
        return match.group(1) if match else None

    def _to_error(self, args: Sequence[str], failure: _process.ProcessError) -> Exception:
        """把 ProcessError 翻译成本工具的错误；默认只补一条缺失提示"""
        if self.translate is not None:
            custom = self.translate(args, failure)
            if custom is not None:
                return custom
        if failure.not_found and self.hint:
            return self.error(f"未找到 {self.program!r}；{self.hint}")
        return self.error(str(failure))
