#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
开发环境准备

两个能力，由 `scripts/setup.py` 入口分派:

- ``check()``  只读体检，不修改任何东西（对应 ``just doctor``）
- ``setup()``  体检通过后写仓库级 git 配置、装依赖、修好脚本导入钩子（对应 ``just setup``）
"""

import re
import shutil
from dataclasses import dataclass
from pathlib import Path

from cli import git, process
from common.constant.paths import (
    DEVELOP_DATA_DIR,
    FRONTEND_DIR,
    FRONTEND_NODE_DIR,
    LOCAL_RELEASE_DIR,
    PROJECT_BASE_DIR,
    RELEASE_DIR,
    SITECUSTOMIZE_TEMPLATE_PATH,
    VENV_DIR,
)
from common.constant.repo import GITHUB_REPO
from common.logger import info, success, warning
from util.fs import relative_path


class EnvError(Exception):
    """工具链缺失或版本过低，或某个准备步骤执行失败"""


@dataclass(frozen=True)
class Requirement:
    """一条工具链要求；``minimum`` 恒为三段，便于与探测结果直接比较"""

    program: str
    minimum: tuple[int, int, int]
    hint: str


# 顺序即体检顺序；下限取实际用到的特性所需（`--ff-only` 之外还有 Tauri 2 与 pnpm 11 的锁文件语义）
REQUIREMENTS: tuple[Requirement, ...] = (
    Requirement("git", (2, 30, 0), "https://git-scm.com/downloads"),
    Requirement("uv", (0, 5, 0), "https://docs.astral.sh/uv/getting-started/installation/"),
    Requirement("just", (1, 0, 0), "https://just.systems/man/en/installation.html"),
    Requirement("node", (22, 0, 0), "https://nodejs.org/"),
    Requirement("pnpm", (11, 0, 0), "https://pnpm.io/installation"),
    Requirement("rustc", (1, 80, 0), "https://rustup.rs/"),
    Requirement("cargo", (1, 80, 0), "https://rustup.rs/"),
)

# 强制写在仓库级的配置: 只影响本仓库，不动用户的全局设置
LOCAL_GIT_CONFIG: tuple[tuple[str, str], ...] = (
    # 拉取只允许快进: 分叉时报错，由人显式决定合并，避免 pull 顺手造出无意义的合并提交
    ("pull.ff", "only"),
    # 合并必须产生合并提交，不允许快进（原则 3）；pull 已由上一行的 pull.ff 覆盖
    ("merge.ff", "false"),
    # 中文路径不转义成 \346\265\213 这类八进制串
    ("core.quotepath", "false"),
    # 远程已删除的分支在 fetch 时一并清理本地跟踪引用
    ("fetch.prune", "true"),
)

# 远端统一使用这个名字：文档里的命令、发布流程与 bump 都按它写，新 clone 默认叫 origin
REMOTE_NAME = "github"

# setup 需要预建的运行期目录：都在 .gitignore 里，全新克隆不带，缺了要等到运行时才炸
DIRECTORIES: tuple[Path, ...] = (
    DEVELOP_DATA_DIR,
    LOCAL_RELEASE_DIR,
    RELEASE_DIR / "CI-CD",
)

# 工具 `--version` 输出里的第一段 x.y[.z]
_VERSION_RE = re.compile(r"(\d+(?:\.\d+)+)")

# 单次版本探测与依赖安装的超时（秒）
PROBE_TIMEOUT = 30
INSTALL_TIMEOUT = 900


def check() -> list[str]:
    """只读体检，返回未通过项的描述；空列表表示全部通过"""
    return [*_toolchain_problems(), *_remote_problems(), *_git_config_problems(), *_dependency_problems()]


def setup() -> None:
    """把仓库准备到可开发状态: 写仓库级 git 配置、预建运行期目录、装依赖、修脚本导入钩子"""
    # 必须先建目录: 日志本身写在 data/logs 下，全新克隆里目录不存在时第一条日志就会失败
    _ensure_directories()

    problems = _toolchain_problems()
    if problems:
        raise EnvError("工具链不满足要求: " + "；".join(problems))

    _ensure_remote()
    _apply_git_config()
    _install_frontend_dependencies()
    _sync_python_dependencies()
    _install_sitecustomize()

    success("环境准备完成，可以执行 `just dev`")


def _probe(requirement: Requirement) -> str:
    """
    探测单个工具，返回它自报的版本串

    :raise EnvironmentError: 未安装、无法解析版本，或低于下限
    """
    program = requirement.program
    try:
        result = process.run([program, "--version"], timeout=PROBE_TIMEOUT)
    except process.ProcessError as error:
        raise EnvError(f"未找到 {program!r}，请先安装: {requirement.hint}") from error

    output = f"{result.stdout or ''}{result.stderr or ''}".strip()
    version = _parse_version(output)
    if version is None:
        raise EnvError(f"{program!r} 的 --version 输出无法解析: {output!r}")

    if version < requirement.minimum:
        raise EnvError(f"{program!r} 版本过低: {_format(version)} < {_format(requirement.minimum)}")
    return _format(version)


def _parse_version(text: str) -> tuple[int, int, int] | None:
    """从版本输出里取第一段数字并补齐三段；`v22.22.2`、`cargo 1.96.0 (…)` 都能吃"""
    match = _VERSION_RE.search(text)
    if match is None:
        return None
    parts = [int(part) for part in match.group(1).split(".")][:3]
    parts += [0] * (3 - len(parts))
    return parts[0], parts[1], parts[2]


def _format(version: tuple[int, ...]) -> str:
    """版本元组 -> `1.96.0`，只取前三段"""
    return ".".join(str(part) for part in version[:3])


def _toolchain_problems() -> list[str]:
    """逐条探测工具链；通过的写 info，不通过的收进返回值"""
    problems: list[str] = []
    for requirement in REQUIREMENTS:
        try:
            info(f"{requirement.program} {_probe(requirement)}")
        except EnvError as error:
            problems.append(str(error))
    return problems


def _remote_problems() -> list[str]:
    """检查远端是否已统一命名为 github 并指向本仓库"""
    remotes = git.remotes()
    if not remotes:
        return []
    if REMOTE_NAME in remotes and _is_our_repo(git.remote_url(REMOTE_NAME)):
        info(f"远端 {REMOTE_NAME} 指向 {GITHUB_REPO}")
        return []
    return [f"远端 {REMOTE_NAME} 未就位（当前远端: {', '.join(remotes)}）；执行 `just setup` 自动归一"]


def _git_config_problems() -> list[str]:
    """检查仓库级配置是否已按 LOCAL_GIT_CONFIG 写入"""
    problems: list[str] = []
    for key, expected in LOCAL_GIT_CONFIG:
        actual = git.get_config(key)
        if actual == expected:
            info(f"git 配置 {key}={actual}")
            continue
        problems.append(f"git 配置 {key} 期望 {expected!r}，实际情况 {actual!r}；执行 `just setup` 写入")
    return problems


def _dependency_problems() -> list[str]:
    """检查依赖与导入钩子是否就位（只看存在性，不联网）"""
    problems: list[str] = []
    if not FRONTEND_NODE_DIR.is_dir():
        problems.append(f"前端依赖未安装: {relative_path(FRONTEND_NODE_DIR)} 不存在；执行 `just setup`")
    if not VENV_DIR.is_dir():
        problems.append(f"Python 虚拟环境未创建: {relative_path(VENV_DIR)} 不存在；执行 `just setup`")
    elif not (VENV_DIR / "sitecustomize.py").is_file():
        problems.append(f"脚本导入钩子缺失: {relative_path(VENV_DIR / 'sitecustomize.py')}；执行 `just setup` 重新复制")
    if not problems:
        info(f"依赖与导入钩子就位: {relative_path(FRONTEND_NODE_DIR)}、{relative_path(VENV_DIR)}")
    return problems


def _ensure_remote() -> None:
    """
    把指向本仓库的远端统一改名为 `github`

    其它地方（文档命令、发布流程、bump）都按 `github` 写，而 `git clone` 只会给出 `origin`；
    只改指向本仓库的那个，别人的 fork 或额外远端一律不动。
    """
    remotes = git.remotes()
    if not remotes:
        warning("仓库没有配置任何远端，跳过远端归一")
        return

    current_url = git.remote_url(REMOTE_NAME)
    if REMOTE_NAME in remotes and _is_our_repo(current_url):
        info(f"远端 {REMOTE_NAME} 已指向 {GITHUB_REPO}")
        return

    candidates = [name for name in remotes if name != REMOTE_NAME and _is_our_repo(git.remote_url(name))]
    if not candidates:
        if REMOTE_NAME in remotes:
            raise EnvError(f"远端 {REMOTE_NAME} 指向的不是 {GITHUB_REPO}（当前 {current_url!r}），请自行确认")
        warning(f"没有远端指向 {GITHUB_REPO}（当前远端: {', '.join(remotes)}），跳过远端归一")
        return

    if REMOTE_NAME in remotes:
        raise EnvError(f"已存在远端 {REMOTE_NAME}（{current_url!r}），无法把 {candidates[0]!r} 改名过去")

    source = candidates[0]
    git.rename_remote(source, REMOTE_NAME)
    success(f"远端 {source} 已重命名为 {REMOTE_NAME}")


def _is_our_repo(url: str | None) -> bool:
    """远端 URL 是否指向本仓库的 GitHub 地址；SSH 与 HTTPS 写法都归一后比 host + owner/name"""
    if not url:
        return False
    parts = re.split(r"[/:]", url.strip().rstrip("/").removesuffix(".git"))
    if len(parts) < 3:
        return False
    host = parts[-3].rsplit("@", 1)[-1].lower()
    repository = f"{parts[-2]}/{parts[-1]}".lower()
    return host == "github.com" and repository == GITHUB_REPO.lower()


def _apply_git_config() -> None:
    """逐条写入仓库级配置；同值重复写入是幂等的"""
    for key, value in LOCAL_GIT_CONFIG:
        git.set_config(key, value)
        info(f"git 配置 {key}={value}")


def _ensure_directories() -> None:
    """预建运行期目录：数据目录、本地发布产物、CI 日志归档（均在 .gitignore 中）"""
    for directory in DIRECTORIES:
        directory.mkdir(parents=True, exist_ok=True)
        info(f"目录就绪: {relative_path(directory, PROJECT_BASE_DIR)}")


def _install_frontend_dependencies() -> None:
    """按 lockfile 安装前端依赖；capture=False 让进度直接打到控制台"""
    info("安装前端依赖: pnpm install --frozen-lockfile")
    try:
        process.run(
            ["pnpm", "install", "--frozen-lockfile"],
            cwd=FRONTEND_DIR,
            timeout=INSTALL_TIMEOUT,
            capture=False,
        )
    except process.ProcessError as error:
        raise EnvError(f"前端依赖安装失败: {error}") from error


def _sync_python_dependencies() -> None:
    """按 uv.lock 同步虚拟环境；首次会下载缺失的 Python 解释器，需要外网"""
    info("同步 Python 依赖: uv sync")
    try:
        process.run(["uv", "sync"], cwd=PROJECT_BASE_DIR, timeout=INSTALL_TIMEOUT, capture=False)
    except process.ProcessError as error:
        raise EnvError(f"Python 依赖同步失败: {error}") from error


def _install_sitecustomize() -> None:
    """
    把被追踪的模板复制进虚拟环境

    .venv 不纳入版本控制，重建虚拟环境后钩子会丢，所以每次 setup 都重做一次
    """
    if not VENV_DIR.is_dir():
        raise EnvError(f"虚拟环境目录 {relative_path(VENV_DIR)} 不存在，uv sync 未成功")
    target = VENV_DIR / "sitecustomize.py"
    try:
        shutil.copyfile(SITECUSTOMIZE_TEMPLATE_PATH, target)
    except OSError as error:
        raise EnvError(f"复制脚本导入钩子失败: {error}") from error
    info(f"已更新脚本导入钩子: {relative_path(target)}")
