from pathlib import Path

_DIR = Path(__file__).parent

# 收集所有 .py 文件，排除自己
CODEGEN_FILES = sorted(p for p in _DIR.rglob("*.py") if p.name != "__init__.py")
