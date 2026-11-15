#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
basedpyright 命令封装
"""

import json
import sys
from pathlib import Path

from cli.tool import Timeout, Tool
from common.constant.paths import PROJECT_BASE_DIR
from common.logger import info


class BasedpyrightError(Exception):
    """basedpyright 无法启动，或检查发现类型错误"""


TOOL = Tool(
    "basedpyright",
    BasedpyrightError,
    cwd=PROJECT_BASE_DIR,
    timeout=Timeout.NORMAL,
    search_dir=Path(sys.executable).parent,
    hint="请执行 `just setup` 同步依赖（basedpyright 随 dev 组依赖安装）",
)


def typecheck() -> None:
    """
    按照 pyproject.toml 中的 [tool.basedpyright] 执行类型检查

    `recommended` 模式下 basedpyright 的 `failOnWarnings` 为 true，
    且它走 GitHub Actions 输出分支时 `--level` 过滤不掉 summary 里的 warningCount，
    于是只要包含 warnings 即会退出 1，因此这里读 `--outputjson` 的 summary

    :raise BasedpyrightError: 存在 error 级诊断，或输出不是合法 JSON
    """
    # 有 warning 时 basedpyright 也会退出 1，成败交给 summary.errorCount
    result = TOOL.run(["--outputjson", "--level", "error"], capture=True, allow=(1,))
    try:
        report = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise BasedpyrightError(f"无法解析 basedpyright 的 JSON 输出: {error}") from error

    summary = report["summary"]
    if summary["errorCount"] == 0:
        # warningCount 不受 --level 过滤，正好用来显示分级收紧的进度
        file_count = summary["filesAnalyzed"]
        warning_count = summary["warningCount"]
        info(f"类型检查通过，{file_count} 个文件中没有任何 error，其中 {warning_count} 条 warning 不计入失败")
        return

    for item in report["generalDiagnostics"]:
        start = item["range"]["start"]
        rule = f" ({item['rule']})" if item.get("rule") else ""
        # 非 tty 时 stdout 是块缓冲，不 flush 会把诊断排在异常消息之后
        print(
            f"{item['file']}:{start['line'] + 1}:{start['character'] + 1} - error: {item['message']}{rule}", flush=True
        )
    raise BasedpyrightError(f"类型检查发现 {summary['errorCount']} 个 error")
