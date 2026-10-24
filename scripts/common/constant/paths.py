#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
项目内路径常量

命名：`*_DIR` 目录、`*_PATH` 单个文件、`*_FILES` 一组文件。
"""

from pathlib import Path

CURRENT = Path(__file__).resolve()

# 根目录标记文件
_ROOT_MARKER = "pyproject.toml"


class PathNotRightError(Exception):
    """项目根目录定位失败"""


def _find_project_base_dir(start: Path) -> Path:
    for parent in start.parents:
        if (parent / _ROOT_MARKER).is_file():
            return parent
    raise PathNotRightError(f"未能从 {str(start)!r} 向上找到含 {_ROOT_MARKER!r} 的项目根目录，脚本必须位于仓库内运行")


PROJECT_BASE_DIR = _find_project_base_dir(CURRENT)

# 顶层目录
FRONTEND_DIR = PROJECT_BASE_DIR / "frontend"
BACKEND_DIR = PROJECT_BASE_DIR / "backend"
SCRIPTS_DIR = PROJECT_BASE_DIR / "scripts"
RESOURCES_DIR = PROJECT_BASE_DIR / "resources"
DOCS_DIR = PROJECT_BASE_DIR / "docs"
WORKFLOW_DIR = PROJECT_BASE_DIR / ".github/workflows"
RELEASE_DIR = PROJECT_BASE_DIR / "release"
DEVELOP_DATA_DIR = PROJECT_BASE_DIR / "data"
ASSETS_DIR = PROJECT_BASE_DIR / "assets"

# 仓库根的四篇文档
README_PATH = PROJECT_BASE_DIR / "README.md"
LICENSE_PATH = PROJECT_BASE_DIR / "LICENSE"
CHANGELOG_PATH = PROJECT_BASE_DIR / "CHANGELOG.md"
RELEASE_NOTES_PATH = PROJECT_BASE_DIR / "RELEASE_NOTES.md"

# 版本号所在文件
PYPROJECT_PATH = PROJECT_BASE_DIR / "pyproject.toml"
UV_LOCK_PATH = PROJECT_BASE_DIR / "uv.lock"
TAURI_CONF_PATH = BACKEND_DIR / "tauri.conf.json5"
CARGO_TOML_PATH = BACKEND_DIR / "Cargo.toml"
CARGO_LOCK_PATH = BACKEND_DIR / "Cargo.lock"
PACKAGE_JSON_PATH = FRONTEND_DIR / "package.json"

# 打包时的资源文件
RESOURCES_FILES = ("README.md", "LICENSE", "CHANGELOG.md", "RELEASE_NOTES.md", "database", "help")

# 含版本号字段、由脚本按正则改写的文件
VERSION_FILES = (
    PYPROJECT_PATH,
    TAURI_CONF_PATH,
    CARGO_TOML_PATH,
    PACKAGE_JSON_PATH,
)

# 不带独立版本号、但会随版本一并变动的锁文件，由对应工具同步
VERSION_LOCK_FILES = (
    CARGO_LOCK_PATH,
    UV_LOCK_PATH,
)

# 构建前后做「干净检查」与「还原」的全部文件 = 上述并集
VERSION_TRACKED_FILES = VERSION_FILES + VERSION_LOCK_FILES

# 仓库维护的版本索引
VERSIONS_INDEX_SOURCE_PATH = RESOURCES_DIR / "update" / "versions.json"

# 依赖 / 构建 / 签名 / 生成物
FRONTEND_NODE_DIR = FRONTEND_DIR / "node_modules"
FRONTEND_DIST_DIR = FRONTEND_DIR / "dist"
BACKEND_TARGET_DIR = BACKEND_DIR / "target"
BACKEND_BUILD_INFO_PATH = BACKEND_DIR / "gen" / "build" / "build_info.json"
LOCAL_RELEASE_DIR = RELEASE_DIR / "Local"
