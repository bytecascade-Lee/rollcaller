#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
把 <项目根>/scripts 注入 sys.path，使 scripts 下的脚本能 import 到同级包。

sitecustomize 是 CPython 启动时由 site 模块自动导入的钩子（见标准库 site.py execsitecustomize），
本文件利用这一机制实现上述注入。Python 只把「脚本自身所在的那一层目录」放进 sys.path[0]，
所以 scripts/xxx.py 天然能 import，而 scripts/a_mudole/a_script.py 不行。

本文件是模板，必须复制到 .venv/sitecustomize.py 才会被加载。parent.parent 是按 .venv 下的位置推导的。

CI 环境下使用环境变量 PYTHONPATH=${{ github.workspace }}/scripts
"""

import sys
from pathlib import Path

_SCRIPTS_DIR = Path(__file__).resolve().parent.parent / "scripts"

if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))
