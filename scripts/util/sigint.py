#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""接管 SIGINT（Ctrl+C）

- graceful_sigint：第一次等待子进程退出，第二次强制终止
- quiet_sigint：收到后安静退出

Ctrl+C 在控制台是广播，同一次按键会被每个进程各响应一次，一个进程只该按自己的角色选择一种。
"""

import signal
import sys
from contextlib import contextmanager
from typing import Callable, Iterator, Optional

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
        warning("执行 Ctrl+C，等待子进程退出...")


@contextmanager
def graceful_sigint(
    on_error: Optional[Callable[[str], None]] = None, exit_code: int = 130
) -> Iterator[Callable[[], bool]]:
    """接管 SIGINT

    第一次等待子进程退干净，第二次放行强制收场。中断态静默收场，
    其余异常原样传播；未给 on_error 时不做兜底，直接抛给调用方。

    仅限主线程，"子进程一起退"只对控制台 Ctrl+C 成立，
    IDE 停止、taskkill 等只发给本进程的信号不会连带子进程。

    不可单线程嵌套使用。

    :param on_error:    错误出口回调；未给则异常原样抛出
    :param exit_code:   中断后的退出码，默认 130
    """
    guard = _InterruptGuard()
    prevent = signal.signal(signal.SIGINT, guard.on_sigint)
    try:
        yield guard.is_interrupted
    except KeyboardInterrupt:
        pass
    except Exception as e:
        if not guard.is_interrupted():
            if on_error is None:
                raise
            on_error(str(e))
            sys.exit(1)
    finally:
        signal.signal(signal.SIGINT, prevent)
    if guard.is_interrupted():
        sys.exit(exit_code)


@contextmanager
def quiet_sigint(exit_code: int = 130) -> Iterator[None]:
    """静默收场

    收到 SIGINT 立刻按 exit_code 退出

    :param exit_code:   退出码，默认 130
    """

    def on_sigint(_signum: int, _frame: object) -> None:
        sys.exit(exit_code)

    prevent = signal.signal(signal.SIGINT, on_sigint)
    try:
        yield
    finally:
        signal.signal(signal.SIGINT, prevent)
