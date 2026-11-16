#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
产物直链的限速
"""

import time
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path
from typing import Self

# 默认限速（KB/s，1024 进制）
DEFAULT_RATE_KB = 4096
# 限速时的分块上限：chunk ≤ rate/2 保证每 tick ≤ 0.5 s
MAX_CHUNK = 64 * 1024
# 不限速时的读块
PLAIN_READ = 256 * 1024


@dataclass(frozen=True, slots=True)
class Throttle:
    """一条产物直链的发送节奏

    :param rate_kb: 目标速率 KB/s（1024 进制）；`<= 0` 表示不限速
    :param chunk: 每次读出的字节数
    :param tick: 每块的目标间隔（秒）；None 表示不限速
    """

    rate_kb: int
    chunk: int
    tick: float | None

    @classmethod
    def of(cls, rate_kb: int) -> Self:
        """由 KB/s 推出分块与 tick"""
        if rate_kb <= 0:
            return cls(rate_kb=rate_kb, chunk=PLAIN_READ, tick=None)
        rate = rate_kb * 1024
        chunk = max(1, min(MAX_CHUNK, rate // 2))
        return cls(rate_kb=rate_kb, chunk=chunk, tick=chunk / rate)

    @property
    def limited(self) -> bool:
        return self.tick is not None

    def describe(self) -> str:
        """限速描述，展示实际分块与 tick"""
        if self.tick is None:
            return "unlimited（--rate 0）"
        return f"{self.rate_kb} KB/s（1024 进制；chunk {self.chunk} B，tick {self.tick * 1000:.1f} ms）"

    def stream(self, target: Path) -> Iterator[bytes]:
        """按计划分块产出文件内容

        必须是**同步**生成器：starlette 用 `iterate_in_threadpool` 跑同步迭代器，
        `time.sleep` 落在工作线程里不阻塞事件循环；写成 `async def` + `time.sleep` 会拖死清单请求。
        """
        with target.open("rb") as handle:
            if self.tick is None:
                while block := handle.read(self.chunk):
                    yield block
                return

            rate = self.chunk / self.tick
            start = time.monotonic()
            sent = 0
            while block := handle.read(self.chunk):
                yield block
                sent += len(block)
                lag = start + sent / rate - time.monotonic()
                if lag > 0:
                    time.sleep(lag)
