#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
启动并回收外部进程
"""

import os
import shutil
import subprocess
from collections.abc import Collection
from pathlib import Path
from typing import Any, Mapping, Sequence

from common.logger import debug

# Windows 下 PATH 搜索时优先尝试的扩展名，顺序即优先级
_WINDOWS_SUFFIXES = (".cmd", ".bat", ".exe")
_ALL_WINDOWS_SUFFIXES = _WINDOWS_SUFFIXES + (".com",)


class ProcessError(Exception):

    def __init__(self, argv: Sequence[str], *, return_code: int | None = None,
                 stdout: str = "", stderr: str = "",
                 timed_out: bool = False, not_found: bool = False) -> None:
        """
        外部进程无法启动、超时，或退出码不在允许集合内。

        :param argv: 逻辑 argv，第一个元素是 program，不是 cmd.exe
        :param return_code: 进程退出码，对应 subprocess.CompletedProcess.returncode；程序未启动时为 None
        :param stdout: 捕获到的标准输出；capture=False 时为空串
        :param stderr: 捕获到的标准错误；capture=False 时为空串
        :param timed_out: 是否因超时被终止
        :param not_found: 是否连可执行文件都没找到
        """
        if not argv:
            raise ValueError("argv 不能为空")
        self.argv = tuple(argv)
        self.returncode = return_code
        self.stdout = stdout
        self.stderr = stderr
        self.timed_out = timed_out
        self.not_found = not_found
        super().__init__(self._describe())

    @property
    def detail(self) -> str:
        return (self.stderr.strip() or self.stdout.strip())[:500]

    def _describe(self) -> str:
        command = " ".join(self.argv)
        detail = f": {self.detail}" if self.detail else ""
        if self.not_found:
            return f"未找到可执行文件 {self.argv[0]!r}，请确认已安装并加入 PATH"
        if self.timed_out:
            return f"{command!r} 执行超时{detail}"
        return f"{command!r} 失败，退出码为 {self.returncode}{detail}"

    def __reduce__(self):
        return self.__class__, (self.argv,), self.__dict__

    def __setstate__(self, state):
        self.__dict__.update(state)
        self.args = (self._describe(),)


def resolve(program: str, *, search_dir: Path | None = None) -> str:
    """解析 program 为实际可执行文件路径

    解析规则按优先级如下：
        - 带路径分隔符或绝对路径：按路径直接使用，不做任何搜索，此时 ``search_dir`` 会被忽略
        - 指定了 ``search_dir``：只在该目录内查找，Windows 上按 .cmd / .bat / .exe 优先级补全扩展名
        - 其余情况：通过系统 ``PATH`` 搜索，Windows 上同样优先尝试特定扩展名

    :param program: 逻辑 program 名称或路径，如 ``"git"``、``"./scripts/build.sh"``
    :param search_dir: 限定搜索目录；None 表示走系统 PATH 搜索，若 program 已带路径分隔符，此参数无效

    :raise ProcessError: 目标文件不存在或未找到（``not_found=True``）
    """
    candidate = Path(program)
    if candidate.is_absolute() or os.sep in program or (os.altsep and os.altsep in program):
        if candidate.is_file():
            return os.path.abspath(str(candidate))
        raise ProcessError([program], not_found=True)

    for name in _candidate_names(program):
        if search_dir is not None:
            found = search_dir / name
            if found.is_file():
                return os.path.abspath(str(found))
        else:
            found = shutil.which(name)
            if found:
                return found
    raise ProcessError([program], not_found=True)


def _candidate_names(program: str) -> tuple[str, ...]:
    """按平台给出候选文件名，Windows 且未带已知扩展名时，优先 .cmd / .bat / .exe"""
    if os.name != "nt":
        return (program,)
    if program.lower().endswith(_ALL_WINDOWS_SUFFIXES):
        return (program,)
    return *[program + suffix for suffix in _WINDOWS_SUFFIXES], program


def build_argv(argv: Sequence[str]) -> list[str]:
    """把逻辑 argv 中的 program 解析成真实路径"""
    if not argv:
        raise ValueError("argv 不能为空")
    program, *rest = argv
    return [resolve(program), *rest]


def run(argv: Sequence[str], *, cwd: Path | None = None, timeout: float | int | None = None,
        env: Mapping[str, str] | None = None, capture: bool = True, text: bool = True,
        stdin: int | None = None, allow: Collection[int] = ()) -> subprocess.CompletedProcess[Any]:
    """执行外部命令

    :param argv:    逻辑 argv，如 ["git", "rev-parse", "HEAD"]
    :param cwd:     工作目录；None 表示继承
    :param timeout: 超时秒数；None 表示不限
    :param env:     完整环境变量映射，替换语义；None 表示继承
    :param capture: 是否捕获 stdout/stderr；False 时直接打印到控制台，返回的 CompletedProcess.stdout/stderr 是 None
    :param text:    True 按 utf-8 解码为 str，False 返回原始 bytes
    :param stdin: 默认继承；CI 传 subprocess.DEVNULL
    :param allow:   **额外**允许的退出码；0 恒允许（"对象不存在"这类约定俗成的非零码放这里）

    :raise ProcessError: 无法启动、超时，或退出码既不是 0 也不在 allow 内
    """
    debug(f"执行: {' '.join(argv)}（cwd={cwd or 'INHERIT'}）")
    exec_argv = build_argv(argv)

    kwargs: dict[str, Any] = {"cwd": cwd, "env": env, "check": False, "stdin": stdin}
    if text:
        kwargs.update(text=True, encoding="utf-8", errors="replace")
    if capture:
        kwargs.update(stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if timeout is not None:
        kwargs["timeout"] = timeout

    if cwd is not None and not Path(cwd).is_dir():
        raise ProcessError(argv, stderr=f"工作目录 {str(cwd)!r} 不存在")
    try:
        result = subprocess.run(exec_argv, **kwargs)
    except FileNotFoundError as error:
        raise ProcessError(argv, not_found=True) from error
    except subprocess.TimeoutExpired as error:
        raise ProcessError(
            argv,
            timed_out=True,
            stdout=_as_text(error.stdout),
            stderr=_as_text(error.stderr),
        ) from error

    if result.returncode != 0 and result.returncode not in allow:
        raise ProcessError(
            argv,
            return_code=result.returncode,
            stdout=_as_text(result.stdout),
            stderr=_as_text(result.stderr),
        )
    return result


def _as_text(value: bytes | str | None) -> str:
    """把超时/失败现场的输出归一成 str"""
    if value is None:
        return ""
    if isinstance(value, bytes):
        return value.decode("utf-8", "replace")
    return value
