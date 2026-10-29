#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""接管 SIGINT（Ctrl+C）

graceful_sigint：第一次记一条警告、让当前工作自然收尾，第二次放行强制中断。
Ctrl+C 在控制台是广播，同一次按键会被每个进程各响应一次，一个进程只该按自己的角色选择一种处理方式。
"""

import signal
import sys
from contextlib import contextmanager
from typing import Callable, Iterator

from common.logger import warning


class _InterruptGuard:
    """一次 graceful_sigint 的中断状态：每次调用独立一份"""

    interrupted: bool

    def __init__(self) -> None:
        self.interrupted = False

    def is_interrupted(self) -> bool:
        return self.interrupted

    def on_sigint(self, _signum: int, _frame: object) -> None:
        if self.interrupted:
            raise KeyboardInterrupt
        self.interrupted = True
        warning("执行 Ctrl+C，正在退出...")


def never_interrupted() -> bool:
    """未被 graceful_sigint 接管时的默认查询：永不中断"""
    return False


@contextmanager
def graceful_sigint(exit_code: int = 130) -> Iterator[Callable[[], bool]]:
    """接管 SIGINT

    第一次让当前工作自然收尾（不打断正在进行的构建 / 子进程），第二次放行强制收场。
    只处理 KeyboardInterrupt，其余异常原样传播（保留原始 traceback），由调用方的顶层 except 负责。
    中断后即使 with 体内以 sys.exit(...) 收场，也以 exit_code 为准。

    仅限主线程，"子进程一起退"只对控制台 Ctrl+C 成立，
    IDE 停止、taskkill 等只发给本进程的信号不会连带子进程。

    不可单线程嵌套使用。

    :param exit_code:   中断后的退出码，默认 130；dev 这类"中断即正常结束"的场景传 0
    """
    guard = _InterruptGuard()
    prevent = signal.signal(signal.SIGINT, guard.on_sigint)
    try:
        yield guard.is_interrupted
    except KeyboardInterrupt:
        pass
    except SystemExit:
        # 中断后 body 可能已经走到 sys.exit(main())，此时以中断码为准
        if not guard.is_interrupted():
            raise
    finally:
        signal.signal(signal.SIGINT, prevent)
    if guard.is_interrupted():
        sys.exit(exit_code)
