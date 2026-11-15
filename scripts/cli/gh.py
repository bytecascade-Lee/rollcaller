#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
GitHub CLI 操作模块，封装常用 gh 命令 / REST 请求，返回结构化数据

说明：
    - API 建 ref 不会产生 push 事件，因此不会唤起监听 push 的 workflow
"""

import json
import re
from collections.abc import Iterable, Sequence
from pathlib import Path
from typing import Any
from urllib.parse import urlencode

from cli import _process
from cli._tool import Timeout, Tool

PAGE_SIZE = 100
MAX_PAGES = 100
_HTTP_STATUS_RE = re.compile(r"HTTP\s+(\d{3})")


class GhError(Exception):
    """gh 操作失败的基类"""

    pass


class GhAuthError(GhError):
    """gh 未登录或缺少凭据"""

    pass


def _translate(args: Sequence[str], failure: _process.ProcessError) -> Exception | None:
    """401 是凭据问题，任何调用点都应立刻可见；403 可能是限流，不并进来"""
    if args[:2] == ["auth", "status"] or http_status_from_text(failure.detail) == 401:
        return GhAuthError(f"gh 鉴权失败: {failure.detail or failure}")
    return None


TOOL = Tool(
    "gh",
    GhError,
    timeout=Timeout.NETWORK,
    hint="请安装 GitHub CLI 并加入 PATH：https://cli.github.com/",
    translate=_translate,
)


def auth() -> None:
    """检查 gh 登录状态"""
    try:
        args = ["auth", "status"]
        _ = TOOL.text(args, timeout=Timeout.NETWORK)
    except GhError as e:
        raise GhAuthError(f"gh 未通过鉴权：{e}，请先运行 gh auth login，或在环境中导出 GH_TOKEN") from e


def api_path(repo: str, endpoint: str) -> str:
    """拼接 REST 路径：repos/{repo}/{endpoint}"""
    return f"repos/{repo}/{endpoint.lstrip('/')}"


def api_get(repo: str, endpoint: str, params: dict[str, Any] | None = None) -> Any:
    """执行一次 gh api GET 并解析 JSON

    :param repo: 仓库名
    :param endpoint: 待请求的端点
    :param params: 发送 GRT 请求时携带的参数

    :raise GhError: 执行失败、空响应、非 JSON
    """
    query = f"?{urlencode(params)}" if params else ""
    url = f"{api_path(repo, endpoint)}{query}"
    args = ["api", url]
    output = TOOL.text(args, timeout=Timeout.NETWORK)
    if not output:
        raise GhError(f"gh api {url} 返回空响应")
    try:
        return json.loads(output)
    except json.JSONDecodeError as error:
        raise GhError(f"gh api {url} 返回非 JSON 响应: {error}") from error


def api_get_or_none(repo: str, endpoint: str, params: dict[str, Any] | None = None) -> Any | None:
    """同 api_get，但 HTTP 404 返回 None"""
    try:
        return api_get(repo, endpoint, params)
    except GhError as error:
        if http_status(error) == 404:
            return None
        raise


def api_pages(repo: str, endpoint: str, list_key: str, params: dict[str, Any] | None = None) -> list[dict[str, Any]]:
    """
    自动翻页拉取列表型响应中的数组字段

    :param repo: 仓库名
    :param endpoint: 待请求的端点
    :param list_key: 响应对象中数组的键，如 workflows / workflow_runs
    :param params: 发送分页请求时携带的参数

    :raise GhError: 分页超过 MAX_PAGES，表表明接口语义变化或服务端异常
    """
    items: list[dict[str, Any]] = []
    for page in range(1, MAX_PAGES + 1):
        body = api_get(repo, endpoint, {**(params or {}), "per_page": PAGE_SIZE, "page": page})
        chunk = body.get(list_key) if isinstance(body, dict) else None
        if not chunk:
            break
        items.extend(chunk)

        total = body.get("total_count") if isinstance(body, dict) else None
        # 末页判据：不足一页，或已达 total_count（无 total_count 的端点靠前者）
        if len(chunk) < PAGE_SIZE:
            break
        if isinstance(total, int) and len(items) >= total:
            break
    else:
        raise GhError(f"gh api {endpoint} 翻页超过 {MAX_PAGES} 页，疑似接口语义变化")
    return items


def ref_commit_sha(repo: str, ref: str) -> str | None:
    """解析分支 / 标签 / 提交对应的提交哈希，不存在返回 None"""
    data = api_get_or_none(repo, f"commits/{ref}")
    if data is None:
        return None
    sha = data.get("sha") if isinstance(data, dict) else None
    if not sha:
        raise GhError(f"gh api commits/{ref} 响应缺少 sha 字段")
    return str(sha)


def create_tag(repo: str, tag: str, sha: str) -> None:
    """创建轻量标签 refs/tags/<tag> 指向 sha（API 建 ref 不会触发 push 事件）"""
    args = ["api", api_path(repo, "git/refs"), "-f", f"ref=refs/tags/{tag}", "-f", f"sha={sha}"]
    _ = TOOL.text(args, timeout=Timeout.NETWORK)


def release_exists(repo: str, tag: str) -> bool:
    """该 tag 是否已有 Release（草稿也算）"""
    return api_get_or_none(repo, f"releases/tags/{tag}") is not None


def create_release(
    repo: str,
    tag: str,
    title: str,
    notes_file: Path,
    *,
    draft: bool,
    prerelease: bool,
    files: Iterable[Path] = (),
) -> None:
    """创建 Release，附件随创建一并上传（tag 须已存在）"""
    args = [
        "release",
        "create",
        tag,
        "--repo",
        repo,
        "--title",
        title,
        "--notes-file",
        str(notes_file),
    ]
    if draft:
        args.append("--draft")
    if prerelease:
        args.append("--prerelease")
    args += [str(path) for path in files]
    _ = TOOL.text(args, timeout=Timeout.EXTENDED)


def edit_release(
    repo: str,
    tag: str,
    title: str,
    notes_file: Path,
    *,
    draft: bool,
    prerelease: bool,
) -> None:
    """
    更新既有 Release 的标题与说明。

    draft / prerelease 都显式传布尔值：重跑时若上一轮是草稿 / 预发布、本轮要转正，
    需要 `--draft=false` / `--prerelease=false` 把状态改回来（两个方向都幂等）。
    """
    args = [
        "release",
        "edit",
        tag,
        "--repo",
        repo,
        "--title",
        title,
        "--notes-file",
        str(notes_file),
        "--draft" if draft else "--draft=false",
        "--prerelease" if prerelease else "--prerelease=false",
    ]
    _ = TOOL.text(args, timeout=Timeout.NETWORK)


def upload_release_assets(repo: str, tag: str, files: Iterable[Path]) -> None:
    """上传附件到既有 Release，同名附件覆盖（--clobber）"""
    args = ["release", "upload", tag, "--repo", repo, "--clobber", *[str(path) for path in files]]
    _ = TOOL.text(args, timeout=Timeout.EXTENDED)


def pr_open_numbers(repo: str, head: str) -> list[int]:
    """该 head 分支上处于打开状态的 PR 编号；没有则返回空列表"""
    args = [
        "pr",
        "list",
        "--repo",
        repo,
        "--head",
        head,
        "--state",
        "open",
        "--json",
        "number",
        "--limit",
        str(PAGE_SIZE),
    ]
    output = TOOL.text(args, timeout=Timeout.NETWORK)
    if not output:
        return []
    try:
        data = json.loads(output)
    except json.JSONDecodeError as error:
        raise GhError(f"gh pr list 返回非 JSON: {error}") from error
    if not isinstance(data, list):
        raise GhError(f"gh pr list 返回的不是数组: {output[:120]!r}")
    return [int(item["number"]) for item in data if isinstance(item, dict) and "number" in item]


def pr_create(repo: str, head: str, base: str, title: str, body: str) -> str:
    """创建 PR，返回 gh 打印的 PR 地址；同 head/base 已有打开 PR 时 gh 会报错"""
    args = [
        "pr",
        "create",
        "--repo",
        repo,
        "--head",
        head,
        "--base",
        base,
        "--title",
        title,
        "--body",
        body,
    ]
    return TOOL.text(args, timeout=Timeout.NETWORK)


def http_status(error: GhError) -> int | None:
    """从 GhError 消息中解析 HTTP 状态码；无法解析返回 None"""
    return http_status_from_text(str(error))


def http_status_from_text(text: str) -> int | None:
    match = _HTTP_STATUS_RE.search(text)
    return int(match.group(1)) if match else None
