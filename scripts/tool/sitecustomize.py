import sys
from pathlib import Path

# sitecustomize 是 CPython 启动时自动导入的钩子（见标准库 site.py 的 execsitecustomize），
# 这里用它把 <项目根>/scripts 注入 sys.path，使 scripts/ 下的脚本都能 import 到 util / cli / common 这些同级包。
#
# 为什么需要它：Python 只把「脚本自身所在的那一层目录」放进 sys.path[0]。
# 所以 scripts/xxx.py（与包同级）天然能 import，而 scripts/codegen/xxx.py 不行。
#
# 位置约束：本文件是**模板**，必须复制到 .venv/sitecustomize.py 才会被加载，
# venv 根目录本身在 sys.path 上，site.py 正是在那里找 sitecustomize。
# 下面的 parent.parent 是按 .venv 下的位置推导的：
#     <项目根>/.venv/sitecustomize.py  ->  parent.parent = <项目根>
# 直接放在 scripts/tool/ 下不会被自动加载，而且在那里算出来的路径会错成 scripts/scripts。

_SCRIPTS_DIR = Path(__file__).resolve().parent.parent / "scripts"

if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))
