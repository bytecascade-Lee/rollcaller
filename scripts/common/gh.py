#!/usr/bin/env python3
"""
GitHub CLI 操作模块：封装常用 gh 命令 / REST 请求，返回结构化数据。

与 git.py / cnb.py 保持相同约定：
- 所有函数不做环境嗅探、不探测默认仓库（仓库以 "owner/repo" 显式传入），
- 失败时抛出异常，由调用方决定如何处理，
- 命令级日志（`执行: gh …`）由本模块统一打印，调用方只报"业务进展"；批量/循环调用
  可传 log_command=False 关掉，由调用方自己报进度（见 download_run_log）。

repo 参数统一形如 "owner/repo"，例如 "bytecascade-Lee/rollcaller"。

目前覆盖的用法：
- require_auth(): gh 鉴权预检
- list_workflows(): 枚举仓库全部 workflow（含 name / id / path）
- list_runs(): 枚举全部 workflow run（自动分页；run 对象含 name / id / run_number /
  status / conclusion / created_at / display_title / event / head_branch）
- download_run_log(): 下载某个 run 的日志 zip（GitHub 返回 302 → 签名 URL，gh 自动跟随）
- http_status(): 从 GhError 中解析 HTTP 状态码，便于区分「日志过期 404」等场景

发布链路（与 common/cnb.py 一一对应）：
- ref_commit_sha(): 解析分支 / 标签 / 提交对应的提交哈希（不存在返回 None）
- create_tag(): 创建轻量标签指向指定提交
- release_exists() / create_release() / edit_release() / upload_release_assets()

说明：
- Actions runs API 的 run 对象里 name 即 workflow 的显示名（Release / Sync Mirrors），
  run_number 是该 workflow 内递增的序号（对应 Actions 页的 Release #N），
  id 是 run 的数据库 ID（下载日志使用的编号，全局唯一）。
- 发布链路用 REST API（/commits/{ref}、/git/refs、/releases/tags/{tag}）而不是 gh release
  子命令：状态码语义明确，可直接用 http_status() 区分「不存在」这类预期否定结果。
  API 建 ref 不会产生 push 事件，因此不会唤起监听 push 的 workflow（sync-mirrors）。
"""

import json
import re
import subprocess
from pathlib import Path
from typing import Dict, Iterable, List, Optional

from common.logger import log

_HTTP_STATUS_RE = re.compile(r"HTTP\s+(\d{3})")
_PAGE_SIZE = 100


class GhError(Exception):
    """gh 操作失败的基类"""
    pass


class GhAuthError(GhError):
    """gh 未登录或缺少凭据"""
    pass


def gh(args: List[str], log_command: bool = True) -> str:
    """
    执行 gh 命令，返回 stdout 字符串（utf-8）。

    Args:
        args: gh 子命令参数，如 ["auth", "status"]、["api", "repos/..."]
        log_command: 是否打印命令级日志（默认打印；循环调用可关掉）
    Raises:
        GhAuthError: gh 未登录
        GhError: 其他 gh 错误
    """
    if log_command:
        log("INFO", f"执行: gh {' '.join(args)}")
    try:
        result = subprocess.run(
            ["gh", *args],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
    except FileNotFoundError:
        raise GhError("未找到 gh 命令，请先安装 GitHub CLI (https://cli.github.com)")

    if result.returncode != 0:
        stderr = result.stderr.strip()
        if http_status_from_text(stderr) is None and "auth" in " ".join(args):
            raise GhAuthError(f"gh 鉴权失败: {stderr}")
        raise GhError(f"gh {' '.join(args)} 失败: {stderr or result.stdout.strip()}")

    return result.stdout.strip()


def gh_bytes(args: List[str], log_command: bool = True) -> bytes:
    """
    执行 gh 命令，返回原始字节 stdout（用于下载二进制内容）。

    Raises:
        GhError: gh 执行失败
    """
    if log_command:
        log("INFO", f"执行: gh {' '.join(args)}")
    try:
        result = subprocess.run(
            ["gh", *args],
            capture_output=True,
            check=False,
        )
    except FileNotFoundError:
        raise GhError("未找到 gh 命令，请先安装 GitHub CLI (https://cli.github.com)")

    if result.returncode != 0:
        stderr = result.stderr.decode("utf-8", errors="replace").strip()
        raise GhError(f"gh {' '.join(args)} 失败: {stderr}")

    return result.stdout


def require_auth() -> None:
    """
    预检 gh 登录状态。

    Raises:
        GhAuthError: 未登录，或未设置 GH_TOKEN
    """
    try:
        gh(["auth", "status"])
    except GhError as e:
        raise GhAuthError(
            f"gh 未通过鉴权预检（{e}）。"
            "请先运行 gh auth login，或在环境中导出 GH_TOKEN。"
        )


def _api_url(repo: str, endpoint: str) -> str:
    """拼接 REST API 路径：repos/{repo}/{endpoint}"""
    return f"repos/{repo}/{endpoint.lstrip('/')}"


def _api_json(repo: str, endpoint: str, params: Optional[Dict] = None) -> Dict:
    """
    执行一次 gh api GET 并解析 JSON 响应。

    Raises:
        GhError: gh 执行失败或响应为空/非 JSON
    """
    query = ""
    if params:
        query = "?" + "&".join(f"{k}={v}" for k, v in params.items())
    url = f"{_api_url(repo, endpoint)}{query}"
    out = gh(["api", url])
    if not out:
        raise GhError(f"gh api {url} 返回空响应")
    try:
        return json.loads(out)
    except json.JSONDecodeError as e:
        raise GhError(f"gh api {url} 返回非 JSON 响应: {e}")


def _api_pages(repo: str, endpoint: str, params: Optional[Dict], list_key: str) -> List[Dict]:
    """
    按 total_count 自动翻页拉取列表型响应中的数组字段。

    Args:
        list_key: 响应对象中数组的键，如 workflows / workflow_runs
    Returns:
        合并后的元素列表（按 API 顺序，run 为最新在前）
    """
    items: List[Dict] = []
    page = 1
    while True:
        body = _api_json(repo, endpoint, {**(params or {}), "per_page": _PAGE_SIZE, "page": page})
        chunk = body.get(list_key) or []
        items.extend(chunk)
        total = body.get("total_count") or 0
        if not chunk or len(items) >= total:
            break
        page += 1
    return items


def list_workflows(repo: str) -> List[Dict]:
    """
    枚举仓库全部 workflow。

    Args:
        repo: "owner/repo"
    Returns:
        workflow 列表，每项含 id / name / path / state 等字段。
        注意：GitHub 内部自动 workflow（如 Dependency Graph）也会出现在这里，
        需要配合 list_dir_names(".github/workflows") 的真实文件白名单来区分。
    """
    return _api_pages(repo, "actions/workflows", {}, "workflows")


def list_dir_names(repo: str, path: str) -> List[str]:
    """
    列出仓库某目录下的文件名（contents API）。

    Args:
        repo: "owner/repo"
        path: 目录路径，如 ".github/workflows"
    Returns:
        文件名列表；目录不存在时返回空列表。
    """
    try:
        out = gh(["api", f"{_api_url(repo, f'contents/{path}')}"])
    except GhError as e:
        if http_status(e) == 404:
            return []
        raise
    try:
        items = json.loads(out)
    except json.JSONDecodeError as e:
        raise GhError(f"gh api contents/{path} 返回非 JSON 响应: {e}")
    if not isinstance(items, list):
        return []
    return [
        item.get("name") for item in items
        if isinstance(item, dict) and item.get("type") == "file" and item.get("name")
    ]


def list_runs(repo: str, limit: Optional[int] = None) -> List[Dict]:
    """
    枚举仓库全部 workflow run（最新在前，自动翻页）。

    Args:
        repo: "owner/repo"
        limit: 只保留最近 N 条（可选）
    Returns:
        run 列表。关键字段：
            id            数据库 ID（下载日志使用）
            run_number    该 workflow 内序号（Actions 页 #N）
            name          workflow 显示名（Release / Sync Mirrors）
            status        当前状态（queued / in_progress / completed 等）
            conclusion    结论（success / failure 等）
            created_at    触发时间（ISO 8601 UTC）
    """
    items = _api_pages(repo, "actions/runs", {}, "workflow_runs")
    if limit is not None:
        items = items[:limit]
    return items


def ref_commit_sha(repo: str, ref: str) -> Optional[str]:
    """
    解析分支 / 标签 / 提交对应的提交哈希；不存在返回 None。

    用 /commits/{ref} 而不是 /git/refs/tags/{tag}：前者对附注标签也会解析出真实提交，
    便于与待发布的提交做比对。

    Args:
        repo: "owner/repo"
        ref: 分支名 / 标签名 / 提交哈希
    Raises:
        GhError: 查询失败（404 除外，404 表示 ref 不存在）
    """
    try:
        data = _api_json(repo, f"commits/{ref}")
    except GhError as e:
        if http_status(e) == 404:
            return None
        raise
    sha = data.get("sha")
    if not sha:
        raise GhError(f"gh api commits/{ref} 响应缺少 sha 字段")
    return str(sha)


def create_tag(repo: str, tag: str, sha: str) -> None:
    """
    创建轻量标签，指向指定提交（refs/tags/<tag>）。

    Args:
        repo: "owner/repo"
        tag: 标签名，如 "v0.8.0"
        sha: 目标提交哈希
    Raises:
        GhError: 创建失败（如标签已存在）
    """
    gh(["api", _api_url(repo, "git/refs"), "-f", f"ref=refs/tags/{tag}", "-f", f"sha={sha}"])


def release_exists(repo: str, tag: str) -> bool:
    """
    判断某个标签是否已有 Release（草稿也算）。

    Args:
        repo: "owner/repo"
        tag: 标签名，如 "v0.8.0"
    Raises:
        GhError: 查询失败（404 除外，404 表示尚无 Release）
    """
    try:
        gh(["api", _api_url(repo, f"releases/tags/{tag}")])
        return True
    except GhError as e:
        if http_status(e) == 404:
            return False
        raise


def create_release(
    repo: str,
    tag: str,
    title: str,
    notes_file: Path,
    draft: bool = False,
    files: Iterable[Path] = (),
) -> None:
    """
    创建 Release，附件随创建一并上传。

    Args:
        repo: "owner/repo"
        tag: 标签名（须已存在，由 create_tag / 外部推送产生）
        title: Release 标题
        notes_file: 说明文件路径
        draft: 是否发布为草稿
        files: 待上传的附件路径
    Raises:
        GhError: 创建失败（如 Release 已存在）
    """
    args = [
        "release", "create", tag,
        "--repo", repo,
        "--title", title,
        "--notes-file", str(notes_file),
    ]
    if draft:
        args.append("--draft")
    args += [str(p) for p in files]
    gh(args)


def edit_release(repo: str, tag: str, title: str, notes_file: Path, draft: bool = False) -> None:
    """
    更新既有 Release 的标题与说明。

    draft=False 时显式传 --draft=false：重跑时若上一轮发的是草稿、本轮要正式发布，
    需要这一步把状态改回来（--draft 与 --draft=false 都是幂等的）。

    Args:
        repo: "owner/repo"
        tag: 标签名
        title: 新的 Release 标题
        notes_file: 新的说明文件路径
        draft: 期望的草稿状态
    """
    gh([
        "release", "edit", tag,
        "--repo", repo,
        "--title", title,
        "--notes-file", str(notes_file),
        "--draft" if draft else "--draft=false",
    ])


def upload_release_assets(repo: str, tag: str, files: Iterable[Path]) -> None:
    """
    上传附件到既有 Release，同名附件覆盖（--clobber）。

    Args:
        repo: "owner/repo"
        tag: 标签名
        files: 待上传的附件路径
    """
    gh(["release", "upload", tag, "--repo", repo, "--clobber", *[str(p) for p in files]])


def download_run_log(repo: str, run_id: int, dest: Path) -> None:
    """
    下载单个 workflow run 的日志，保存为 zip。

    GitHub 的 /actions/runs/{id}/logs 返回 302 到签名 URL，gh api 自动跟随；
    下载完成后校验 zip 魔数，防止空响应伪成功。
    该函数通常被逐条循环调用，命令级日志交给调用方（log_command=False）。

    Args:
        repo: "owner/repo"
        run_id: run 的数据库 ID
        dest: 目标 zip 路径（父目录不存在会自动创建）

    Raises:
        GhError: 下载失败或内容不是 zip（日志过期时消息含 HTTP 404/410）
    """
    dest.parent.mkdir(parents=True, exist_ok=True)
    data = gh_bytes(["api", _api_url(repo, f"actions/runs/{run_id}/logs")], log_command=False)
    if not data.startswith(b"PK\x03\x04"):
        raise GhError(f"run {run_id} 的日志下载结果不是有效的 zip（可能尚未生成完整日志）")
    dest.write_bytes(data)


def http_status(error: GhError) -> Optional[int]:
    """
    从 GhError 消息中解析 HTTP 状态码；无法解析时返回 None。

    典型用途：区分「日志超过保留期」的 404/410 与网络等其他错误。
    """
    return http_status_from_text(str(error))


def http_status_from_text(text: str) -> Optional[int]:
    match = _HTTP_STATUS_RE.search(text)
    return int(match.group(1)) if match else None
