#!/usr/bin/env python3
"""
发布线约束：tag 必须打在它上面，它也是 tag 发布时嵌进二进制的来源分支。

引用方：
    scripts/check_release_source.py   —— validate 阶段断言 tag 位于其历史
    scripts/generate_build_info.py    —— tag 发布时把来源分支记为它

注：scripts/tag_release.py 另有「打 tag 时必须在 master/main」的本地前置校验，
尚未收敛到本常量（它额外兼容 main）。
"""

RELEASE_LINE = "master"
