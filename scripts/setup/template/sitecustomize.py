#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
EN-US:
    Inject <project-root>/scripts into sys.path so scripts in nested directories
    can import sibling packages/modules under scripts/.

    Why this works:
    - ``sitecustomize.py`` is a CPython startup hook. The stdlib ``site`` module
      tries to import it automatically (see ``site.execsitecustomize``).
    - Python only puts the directory of the executed script into ``sys.path[0]``.
      Therefore, ``scripts/xxx.py`` can import its neighbors naturally, but
      ``scripts/a_module/a_script.py`` cannot import packages directly under
      ``scripts/`` without this injection.

    Template usage:
    - This file is a template. Copy it to a location from which Python can
      import ``sitecustomize``. For example, this repository may use
      ``.venv/sitecustomize.py``; if your Python does not import from that
      location, put it in the venv's ``site-packages/sitecustomize.py`` instead.
    - The path discovery below searches upward from this file, so it does not
      rely on a fixed ``parent.parent`` layout.
    - In CI, an equivalent setup is:
      ``PYTHONPATH=${{ github.workspace }}/scripts``

ZH-CN:
    将 <项目根>/scripts 注入 sys.path，使 scripts 下更深层子目录中的脚本也能
    import 到 scripts/ 下的同级包/模块。

    原理：
    - ``sitecustomize.py`` 是 CPython 启动钩子，标准库 ``site`` 会自动尝试
      导入它（见 ``site.execsitecustomize``）。
    - Python 只会把「被执行的脚本所在目录」放进 ``sys.path[0]``。所以
      ``scripts/xxx.py`` 天然能 import 邻居，但
      ``scripts/a_module/a_script.py`` 无法直接 import ``scripts/`` 下的包，
      除非做本注入。

    模板用法：
    - 本文件是模板，必须复制到 Python 启动时能 import ``sitecustomize`` 的
      位置，``just setup`` 会自动完成此任务。
    - 下面的路径发现逻辑会从本文件向上搜索，因此不依赖固定的 ``parent`` 层级。
    - CI 中使用等价方式 ``PYTHONPATH=${{ github.workspace }}/scripts``
"""

import sys
from pathlib import Path


def _find_scripts_dir():
    """
    EN-US:
        Find the nearest ``pyproject.toml`` by walking upward from this file,
        treat its parent directory as the project root, and return
        ``<project-root>/scripts`` if that directory exists.

        This is intentionally not hard-coded as ``parent.parent / "scripts"``:
        it also works when this template is copied into
        ``.venv/lib/pythonX.Y/site-packages/sitecustomize.py``.

    ZH-CN:
        从本文件所在位置向上查找最近的 ``pyproject.toml``，将其父目录视为项目根，
        若 ``<项目根>/scripts`` 目录存在则返回该目录。

        这里有意不硬编码 ``parent.parent / "scripts"``：即使模板被复制到
        ``.venv/lib/pythonX.Y/site-packages/sitecustomize.py`` 也能找到项目根。

    Returns:
        pathlib.Path | None:
            EN: The scripts directory if found; otherwise ``None``.
            ZH: 找到则返回 scripts 目录，否则返回 ``None``。
    """
    here = Path(__file__).resolve()

    # EN-US: ``here.parents`` includes the direct parent up to the filesystem root.
    # ZH-CN: ``here.parents`` 包含直接父目录，一直向上到文件系统根目录。
    for parent in here.parents:
        pyproject = parent / "pyproject.toml"

        # EN-US: Stop at the nearest pyproject.toml; that directory is the project root.
        # ZH-CN: 找到最近的 pyproject.toml 即视为项目根，不再继续向上。
        if pyproject.is_file():
            candidate = parent / "scripts"

            # EN-US: Only return a real directory, never a non-existent path.
            # ZH-CN: 只返回真实存在的目录，不返回不存在的路径。
            if candidate.is_dir():
                return candidate

            return None

    return None


_SCRIPTS_DIR = _find_scripts_dir()

if _SCRIPTS_DIR is not None:
    _scripts_dir_str = str(_SCRIPTS_DIR)

    if _scripts_dir_str not in sys.path:
        # EN-US: insert(0) keeps project scripts at highest priority. If you want
        #     to avoid shadowing stdlib modules, use sys.path.append(...) instead.
        # ZH-CN: insert(0) 让项目 scripts 保持最高优先级。如果担心遮蔽标准库模块，
        #     可改为 sys.path.append(...)。
        sys.path.insert(0, _scripts_dir_str)
