#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
开发环境准备

两个能力，由 `scripts/setup.py` 入口分派:

- ``check()``  只读体检，不修改任何东西（对应 ``just doctor``）
- ``setup()``  体检通过后写仓库级 git 配置、装依赖、修好脚本导入钩子（对应 ``just setup``）
"""

import os
import re
import shutil
import tomllib
from dataclasses import dataclass
from pathlib import Path

from cli import git, pnpm, process, rust, uv
from common.constant.paths import (
    CARGO_CONFIG_DIR,
    CARGO_CONFIG_PATH,
    CARGO_CONFIG_TEMPLATE_PATH,
    DEVELOP_DATA_DIR,
    FRONTEND_NODE_DIR,
    FRONTEND_TYPES_DIR,
    LOCAL_RELEASE_DIR,
    PROJECT_BASE_DIR,
    PYPROJECT_PATH,
    RELEASE_DIR,
    SITECUSTOMIZE_TEMPLATE_PATH,
    VENV_DIR,
)
from common.constant.repo import GITHUB_REPO
from common.logger import info, success, warning
from util.fs import relative_path
from util.str_format import join_repr


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

# uv 的默认源只允许声明在 pyproject.toml；这几个位置一旦指向别的源，uv 会重写 uv.lock
UV_DEFAULT_INDEX_ENV = "UV_DEFAULT_INDEX"
UV_CONFIG_FILE_ENV = "UV_CONFIG_FILE"
# 只有 http(s) 值才是源地址，`default-index = "tsinghua"` 这类按名字引用不算
_URL_PREFIXES = ("http://", "https://")

# setup 需要预建的运行期目录：都在 .gitignore 里，全新克隆不带，缺了要等到运行时才炸
DIRECTORIES: tuple[Path, ...] = (
    DEVELOP_DATA_DIR,
    LOCAL_RELEASE_DIR,
    RELEASE_DIR / "CI-CD",
)

# 工具 `--version` 输出里的第一段 x.y[.z]
_VERSION_RE = re.compile(r"(\d+(?:\.\d+)+)")

# cargo 本地配置里必须成立的两项：类型导出目录，与开发模式使用的数据库文件名
CARGO_EXPORT_KEY = "TS_RS_EXPORT_DIR"
CARGO_DATABASE_KEY = "DEVELOP_DATABASE_FILENAME"

# 单次版本探测的超时（秒）；依赖安装的超时由各 cli 模块自己持有
PROBE_TIMEOUT = 30


def check() -> list[str]:
    """只读体检，返回未通过项的描述；空列表表示全部通过"""
    return [*_toolchain_problems(), *_remote_problems(), *_uv_index_problems(),
            *_git_config_problems(), *_cargo_config_problems(), *_dependency_problems()]


def setup() -> None:
    """把仓库准备到可开发状态: 写仓库级配置、预建目录、核对源、下载前后端依赖、修导入钩子"""
    # 必须先建目录: 日志本身写在 data/logs 下，全新克隆里目录不存在时第一条日志就会失败
    _ensure_directories()

    problems = _toolchain_problems()
    if problems:
        raise EnvError("工具链不满足要求: " + "；".join(problems))

    _ensure_remote()
    _apply_git_config()
    _install_cargo_config()
    _ensure_uv_index()
    _warn_if_lock_stale()
    _install_frontend_dependencies()
    _fetch_backend_dependencies()
    _sync_python_dependencies()
    _install_sitecustomize()

    success("环境准备完成，可以执行 `just dev`")


def _probe(requirement: Requirement) -> str:
    """探测单个工具，返回它自报的版本串"""
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


def _cargo_config_problems() -> list[str]:
    """检查 backend/.cargo/config.toml 是否就位，且两项关键配置指向正确位置"""
    label = relative_path(CARGO_CONFIG_PATH, PROJECT_BASE_DIR)
    if not CARGO_CONFIG_PATH.is_file():
        return [f"cargo 本地配置缺失: {label}；执行 `just setup` 从模板复制"]

    try:
        data = tomllib.loads(CARGO_CONFIG_PATH.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        return [f"读取 {label} 失败: {error}"]

    variables = data.get("env")
    if not isinstance(variables, dict):
        return [f"{label} 缺少 [env] 段；删除该文件后执行 `just setup` 可重新生成"]

    export_dir = _cargo_export_dir(variables.get(CARGO_EXPORT_KEY))
    if export_dir is None:
        return [f"{label} 的 {CARGO_EXPORT_KEY} 缺失或写法无法识别，可参照 "
                f"{relative_path(CARGO_CONFIG_TEMPLATE_PATH, PROJECT_BASE_DIR)} 里的写法"]
    expected = FRONTEND_TYPES_DIR.resolve()
    if export_dir != expected:
        return [f"{label} 的 {CARGO_EXPORT_KEY} 指向 {str(export_dir)!r}，"
                f"应为 {relative_path(FRONTEND_TYPES_DIR, PROJECT_BASE_DIR)!r}；否则类型会被导出到别处"]

    database = variables.get(CARGO_DATABASE_KEY)
    if not isinstance(database, str) or not database.strip():
        return [f"{label} 的 {CARGO_DATABASE_KEY} 为空，开发模式启动时会断言失败"]

    info(f"cargo 本地配置就位: {label}")
    return []


def _cargo_export_dir(value: object) -> Path | None:
    """解出 env 里的路径项：支持 {value, relative = true} 与绝对路径字符串，其余一律视为无效"""
    if isinstance(value, dict):
        raw = value.get("value")
        if not isinstance(raw, str):
            return None
        # relative = true 的锚点是「含 .cargo 的那层目录」，即 BACKEND_DIR
        return (CARGO_CONFIG_DIR.parent / raw).resolve() if value.get("relative") else Path(raw)
    if isinstance(value, str):
        path = Path(value)
        return path if path.is_absolute() else None
    return None


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


def _install_cargo_config() -> None:
    """缺失时从模板原样复制；已存在时跳过，避免覆盖本机的私有配置"""
    if CARGO_CONFIG_PATH.is_file():
        info(f"cargo 本地配置已存在，跳过: {relative_path(CARGO_CONFIG_PATH, PROJECT_BASE_DIR)}")
        return
    try:
        CARGO_CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(CARGO_CONFIG_TEMPLATE_PATH, CARGO_CONFIG_PATH)
    except OSError as error:
        raise EnvError(f"复制 cargo 本地配置失败: {error}") from error
    info(f"已从模板生成 cargo 本地配置: {relative_path(CARGO_CONFIG_PATH, PROJECT_BASE_DIR)}")


def _ensure_directories() -> None:
    """预建运行期目录：数据目录、本地发布产物、CI 日志归档（均在 .gitignore 中）"""
    for directory in DIRECTORIES:
        directory.mkdir(parents=True, exist_ok=True)
        info(f"目录就绪: {relative_path(directory, PROJECT_BASE_DIR)}")


def _uv_index_problems() -> list[str]:
    """uv 的默认源必须与 pyproject.toml 声明的一致，否则 uv.lock 会被按别的源重写"""
    project_index = _project_uv_index()
    if project_index is None:
        return []

    conflicts = _conflicting_indexes(project_index)
    if not conflicts:
        info(f"uv 源与本仓库一致: {project_index}")
        return []
    return [
        f"{item}={url}，与本仓库声明的 {project_index} 不一致；源不一致会让 uv 在 sync 时重写 uv.lock"
        for item, url in conflicts
    ]


def _ensure_uv_index() -> None:
    """源不一致直接失败"""
    problems = _uv_index_problems()
    if problems:
        raise EnvError("uv 源配置与本仓库冲突: " + join_repr(problems))


def _project_uv_index() -> str | None:
    """从 pyproject.toml 读 [[tool.uv.index]] 中 default = true 的地址"""
    try:
        data = tomllib.loads(PYPROJECT_PATH.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise EnvError(f"读取 {relative_path(PYPROJECT_PATH)} 失败: {error}") from error

    indexes = data.get("tool", {}).get("uv", {}).get("index", [])
    if isinstance(indexes, dict):
        indexes = [indexes]
    if not indexes:
        return None
    default = [entry for entry in indexes if isinstance(entry, dict) and entry.get("default")]
    chosen = default[0] if default else indexes[0]
    url = chosen.get("url") if isinstance(chosen, dict) else None
    return url if isinstance(url, str) else None


def _conflicting_indexes(project_index: str) -> list[tuple[str, str]]:
    """收集本机上指向别的源的 uv 配置，返回 (来源描述, 地址)"""
    expected = _normalize_index(project_index)
    conflicts: list[tuple[str, str]] = []

    env_value = os.environ.get(UV_DEFAULT_INDEX_ENV)
    if env_value and _normalize_index(env_value) != expected:
        conflicts.append((f"环境变量 {UV_DEFAULT_INDEX_ENV}", env_value))

    for path in _uv_config_candidates():
        if not path.is_file():
            continue
        label = relative_path(path, PROJECT_BASE_DIR)
        try:
            data = tomllib.loads(path.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as error:
            raise EnvError(f"读取 uv 配置 {label} 失败: {error}") from error
        for url in _declared_default_indexes(data):
            if _normalize_index(url) != expected:
                conflicts.append((f"{label} 的默认源", url))
    return conflicts


def _declared_default_indexes(data: dict) -> list[str]:
    """从一份 uv 配置里取出「默认源」的地址；按名字引用别处 index 的写法不算"""
    values: list[str] = []
    for key in ("default-index", "index-url"):
        value = data.get(key)
        if isinstance(value, str) and value.startswith(_URL_PREFIXES):
            values.append(value)

    entries = data.get("index") or []
    if isinstance(entries, dict):
        entries = [entries]
    for entry in entries:
        if isinstance(entry, dict) and entry.get("default"):
            url = entry.get("url")
            if isinstance(url, str) and url.startswith(_URL_PREFIXES):
                values.append(url)
    return values


def _uv_config_candidates() -> list[Path]:
    """uv 会读取的配置文件：显式指定的优先，其次是平台用户配置，最后是仓库根的同名文件"""
    explicit = os.environ.get(UV_CONFIG_FILE_ENV)
    if explicit:
        return [Path(explicit)]

    candidates: list[Path] = []
    if os.name == "nt":
        appdata = os.environ.get("APPDATA")
        if appdata:
            candidates.append(Path(appdata) / "uv" / "uv.toml")
    else:
        xdg = os.environ.get("XDG_CONFIG_HOME")
        base = Path(xdg) if xdg else Path.home() / ".config"
        candidates.append(base / "uv" / "uv.toml")
    candidates.append(PROJECT_BASE_DIR / "uv.toml")
    return candidates


def _normalize_index(url: str) -> str:
    """比较前归一：去首尾空白与结尾斜杠，大小写不敏感"""
    return url.strip().rstrip("/").lower()


def _warn_if_lock_stale() -> None:
    """
    uv.lock 与 pyproject.toml 是否一致

    只警告不失败：锁过期属于仓库状态问题，应当回到开发流程里跑 `uv lock` 并提交；
    而且 `sync --frozen` 仍能按旧锁装出可用的环境，没必要因此卡住整台机器的准备。
    """
    try:
        fresh = uv.lock_check()
    except uv.UvError as error:
        warning(f"无法校验 uv.lock，跳过: {error}")
        return

    if fresh:
        info("uv.lock 与 pyproject.toml 一致")
        return
    warning("uv.lock 已过期，依赖集合可能与仓库声明不同；请运行 `uv lock` 并提交结果")


def _install_frontend_dependencies() -> None:
    """按 lockfile 安装前端依赖"""
    info("安装前端依赖: pnpm install --frozen-lockfile")
    try:
        pnpm.install()
    except pnpm.PnpmError as error:
        raise EnvError(f"前端依赖安装失败: {error}") from error


def _fetch_backend_dependencies() -> None:
    """下载 Rust 依赖"""
    info("下载后端依赖: cargo fetch --locked")
    try:
        rust.fetch()
    except rust.RustError as error:
        raise EnvError(f"后端依赖下载失败: {error}") from error


def _sync_python_dependencies() -> None:
    """按 uv.lock 同步虚拟环境"""
    info("同步 Python 依赖: uv sync --frozen")
    try:
        uv.sync_frozen()
    except uv.UvError as error:
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
