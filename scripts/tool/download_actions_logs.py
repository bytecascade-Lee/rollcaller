#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
下载 GitHub Actions workflow run 的运行日志，按 workflow 分目录归档。

命名规则：
    <workflow 显示名>#<run 序号>-<run 数据库 ID>.zip
    - run 序号：该 workflow 内递增的 run_number（对应 Actions 页 Release #N）
    - run 数据库 ID：下载日志使用的编号，全局唯一

特性：
    - 幂等：目标 zip 已存在则跳过，增量同步，--force 强制重下
    - 容错：单条失败记 ERROR 继续，结尾汇总各 workflow 的成功 / 跳过 / 失败数
    - 预检 gh 登录，未登录给出明确指引
    - 默认只处理 .github/workflows 下真实 workflow 的 run；
      GitHub 内部自动 workflow 默认跳过，--include-gh-runs 可放开

用法:
    uv run ./scripts/tool/download_actions_logs.py
    uv run ./scripts/tool/download_actions_logs.py --workflow Release
    uv run ./scripts/tool/download_actions_logs.py --since 2026-09-01 --dry-run
    uv run ./scripts/tool/download_actions_logs.py --include-running --force
"""

import argparse
import os
import re
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence

from cli import gh, git
from common.constant.paths import RELEASE_DIR
from common.logger import error, info, success, warning
from util import fs

OUT_DIR = RELEASE_DIR / "CI-CD" / "logs"

# 解析 git remote URL 中的 owner/repo（支持 https 与 git@github.com: 两种形式）
_REPO_RE = re.compile(r"github\.com[:/]([^/:]+)/([^/.]+?)(?:\.git)?/?$")


class LogsError(Exception):
    """无法确定仓库，或过滤后没有可处理的 run"""


@dataclass(frozen=True, slots=True)
class Run:
    """一条 workflow run：只保留归档需要的字段

    Actions 的 run 对象里 name 是 run 自身的标题（不是 workflow 名），
    workflow 显示名要按 workflow_id 去 workflows API 查；run_number 是该
    workflow 内递增的序号，id 是全局唯一的数据库 ID（下载日志用）。
    """

    id: int
    number: int
    workflow_id: int | None
    display: str
    status: str
    created_at: str


def _parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="下载 GitHub Actions workflow run 日志并归档到 release/CI-CD/logs",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "示例:\n"
            "  uv run ./scripts/tool/download_actions_logs.py\n"
            "  uv run ./scripts/tool/download_actions_logs.py --workflow Release\n"
            "  uv run ./scripts/tool/download_actions_logs.py --since 2026-09-01 --dry-run\n"
        ),
    )
    parser.add_argument(
        "--workflow", action="append", default=[], metavar="NAME",
        help="只处理指定显示名的 workflow（如 Release），可重复传参；缺省为全部",
    )
    parser.add_argument(
        "--since", metavar="YYYY-MM-DD",
        help="只处理该日期（含）之后触发的 run",
    )
    parser.add_argument(
        "--include-running", action="store_true",
        help="包含未完成的 run（默认只下 status=completed）",
    )
    parser.add_argument(
        "--include-gh-runs", action="store_true",
        help="也处理 GitHub 内部自动 workflow 的 run（如 Dependency Graph），默认跳过",
    )
    parser.add_argument("--limit", type=int, help="只取最近 N 条 run（仓库维度）")
    parser.add_argument(
        "--interval", type=float, default=0.2, metavar="SEC",
        help="每次下载间隔秒数，避免触发限流（默认 0.2）",
    )
    parser.add_argument("--force", action="store_true", help="覆盖已存在的 zip")
    parser.add_argument("--dry-run", action="store_true", help="只打印将执行的动作，不落盘")
    return parser.parse_args(argv)


def _repo_from_url(url: str) -> str | None:
    """从 git remote URL 提取 owner/repo；非 GitHub 地址返回 None"""
    match = _REPO_RE.search(url)
    return f"{match.group(1)}/{match.group(2)}" if match else None


def _resolve_repo() -> str:
    """解析目标仓库：GITHUB_REPOSITORY > git remote(github → origin)"""
    env_repo = os.environ.get("GITHUB_REPOSITORY")
    if env_repo:
        return env_repo

    for remote in ("github", "origin"):
        url = git.remote_url(remote)
        if not url:
            continue
        repo = _repo_from_url(url)
        if repo:
            info(f"从 git remote {remote} 解析到仓库: {repo}")
            return repo

    raise LogsError(
        "无法确定目标仓库：请设置 GITHUB_REPOSITORY 环境变量，"
        "或为本仓库配置 github / origin 远端"
    )


# GitHub API 薄封装：调用点都在本工具，cli.gh 只提供 api_get / api_pages 原语

def _list_runs(repo: str, limit: int | None = None) -> list[dict]:
    """枚举仓库全部 workflow run，最新在前，自动翻页"""
    items = gh.api_pages(repo, "actions/runs", "workflow_runs")
    return items[:limit] if limit is not None else items


def _list_workflows(repo: str) -> list[dict]:
    """枚举仓库全部 workflow"""
    return gh.api_pages(repo, "actions/workflows", "workflows")


def _list_dir_names(repo: str, path: str) -> list[str]:
    """列出仓库某目录下的文件名"""
    items = gh.api_get_or_none(repo, f"contents/{path}")
    if items is None:
        return []
    if not isinstance(items, list):
        raise gh.GhError(f"gh api contents/{path} 返回结构不是数组")
    return [
        item["name"] for item in items
        if isinstance(item, dict) and item.get("type") == "file" and item.get("name")
    ]


def _real_workflow_ids(repo: str, workflows: Sequence[dict]) -> set[int] | None:
    """返回 `.github/workflows` 下真实文件的 workflow id 集合；无法判定时返回 None

    workflows API 也包含 GitHub 内部自动 workflow（如 Dependency Graph，其 run 的
    name 是 "Graph Update: ..." 这类更新标题而非 workflow 名）。因此用仓库目录里
    的真实文件做白名单最稳；目录读不到 / 为空 / 与 API 路径无交集时退化为「不过滤」，
    由调用方保留全部 run 并告警。
    """
    all_ids = {w["id"] for w in workflows if w.get("id") is not None}
    try:
        names = set(_list_dir_names(repo, ".github/workflows"))
    except gh.GhError as e:
        warning(f"读取 .github/workflows 目录失败，本次不过滤内部 workflow: {e}")
        return None
    if not names:
        warning(".github/workflows 目录为空，本次不过滤内部 workflow")
        return None

    paths = {f".github/workflows/{name}" for name in names}
    real_ids = {w["id"] for w in workflows if w.get("id") is not None and w.get("path") in paths}
    if not real_ids:
        warning("workflows API 路径与 .github/workflows 文件无交集，退化为不过滤")
        return None
    return real_ids


def _to_run(payload: dict, names: dict[int, str]) -> Run | None:
    """把 API 的 run 对象转成 Run；缺 id / run_number 时返回 None"""
    run_id = payload.get("id")
    number = payload.get("run_number")
    if not isinstance(run_id, int) or not isinstance(number, int):
        return None

    workflow_id = payload.get("workflow_id")
    return Run(
        id=run_id,
        number=number,
        workflow_id=workflow_id if isinstance(workflow_id, int) else None,
        display=names.get(workflow_id) or payload.get("name") or "unknown",
        status=str(payload.get("status") or ""),
        created_at=str(payload.get("created_at") or ""),
    )


def _should_include(run: Run, args: argparse.Namespace) -> bool:
    """按 --workflow / 状态 / --since 过滤单个 run"""
    if args.workflow and run.display not in args.workflow:
        return False
    if not args.include_running and run.status != "completed":
        return False
    if args.since and run.created_at[:10] < args.since:
        return False
    return True


def _group(runs: Sequence[Run]) -> dict[str, list[Run]]:
    """按 workflow 显示名分组，组内按 run 序号升序"""
    groups: dict[str, list[Run]] = {}
    for run in runs:
        groups.setdefault(run.display, []).append(run)
    for items in groups.values():
        items.sort(key=lambda item: item.number)
    return groups


def _select_runs(repo: str, args: argparse.Namespace) -> list[Run]:
    """枚举全部 run，剔除 GitHub 内部 workflow，再按 CLI 条件筛选"""
    info("正在枚举 workflow runs ...")
    payloads = _list_runs(repo, limit=args.limit)

    workflows = _list_workflows(repo)
    names = {w["id"]: w["name"] for w in workflows if w.get("id") is not None}
    if not names:
        warning("workflows API 未返回任何 workflow，退化为按 run 自身字段分组")

    real_ids = _real_workflow_ids(repo, workflows)
    runs: list[Run] = []
    skipped_internal = 0
    for payload in payloads:
        run = _to_run(payload, names)
        if run is None:
            warning(f"run 缺 id/run_number 字段，跳过（name={payload.get('name')!r}）")
            continue
        if not args.include_gh_runs and real_ids is not None and run.workflow_id not in real_ids:
            skipped_internal += 1
            continue
        runs.append(run)

    if skipped_internal:
        info(f"跳过 {skipped_internal} 条 GitHub 内部自动 workflow 的 run"
             f"（如 Dependency Graph，可用 --include-gh-runs 放开）")

    if args.workflow:
        info(f"已过滤 workflow: {', '.join(args.workflow)}")
    if args.since:
        info(f"已过滤起始日期: {args.since}")
    if not args.include_running:
        skipped_running = sum(1 for run in runs if run.status != "completed")
        if skipped_running:
            info(f"跳过 {skipped_running} 条未完成的 run（可用 --include-running 包含）")

    return [run for run in runs if _should_include(run, args)]


def _new_stats() -> dict[str, int]:
    return {"downloaded": 0, "skipped": 0, "failed": 0, "expired": 0}


def _download_run_log(repo: str, run_id: int, dest: Path) -> None:
    """下载单个 run 的日志并落盘为 zip

    /actions/runs/{id}/logs 会 302 到签名 URL，gh api 自动跟随；
    下载完成后校验 zip 魔数，避免空响应 / 错误页被当成日志。

    :raise gh.GhError: 下载失败或内容不是 zip（日志过期时消息含 HTTP 404/410）
    """
    data = gh.stdout_bytes(["api", gh.api_path(repo, f"actions/runs/{run_id}/logs")])
    if not data.startswith(b"PK\x03\x04"):
        raise gh.GhError(f"run {run_id} 的日志下载结果不是有效的 zip（可能尚未生成完整日志）")
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_bytes(data)


def _download_groups(repo: str, groups: dict[str, list[Run]], args: argparse.Namespace) -> dict[str, int]:
    """逐 workflow 下载，返回各状态的总数"""
    totals = _new_stats()
    for wf_name, wf_runs in groups.items():
        safe = fs.sanitize_filename(wf_name)
        wf_dir = OUT_DIR / safe
        stats = _new_stats()
        info(f"--- workflow: {wf_name}（{len(wf_runs)} 条）---")

        for run in wf_runs:
            dest = wf_dir / f"{safe}#{run.number}-{run.id}.zip"
            if dest.exists() and not args.force:
                info(f"{dest.name} 已存在，跳过")
                stats["skipped"] += 1
                continue
            if args.dry_run:
                info(f"[dry-run] 将下载 {dest.name}")
                stats["downloaded"] += 1
                continue

            try:
                _download_run_log(repo, run.id, dest)
            except gh.GhError as e:
                code = gh.http_status(e)
                if code in (404, 410):
                    stats["expired"] += 1
                    warning(f"{dest.name}: 日志不可用（HTTP {code}，可能超过保留期或尚未生成）")
                else:
                    stats["failed"] += 1
                    error(f"{dest.name}: 下载失败: {e}")
                continue
            except OSError as e:
                stats["failed"] += 1
                error(f"{dest.name}: 写入失败: {e}")
                continue

            stats["downloaded"] += 1
            info(f"已下载 {dest.name}")
            if args.interval > 0:
                time.sleep(args.interval)

        info(f"--- {wf_name}: 下载 {stats['downloaded']} / 跳过 {stats['skipped']}"
             f" / 过期 {stats['expired']} / 失败 {stats['failed']} ---")
        for key in totals:
            totals[key] += stats[key]
    return totals


def main(argv: Sequence[str] | None = None) -> int:
    args = _parse_args(argv)

    repo = _resolve_repo()
    info(f"目标仓库: {repo}")
    info(f"输出目录: {OUT_DIR}")

    gh.auth()
    info("gh 鉴权预检通过")

    selected = _select_runs(repo, args)
    groups = _group(selected)
    if not groups:
        raise LogsError("没有符合条件（含日期/状态/workflow 过滤）的 run")

    info(f"共 {len(selected)} 条 run 待处理，按 {len(groups)} 个 workflow 分组")
    if args.workflow:
        missing = sorted(set(args.workflow) - set(groups))
        for name in missing:
            warning(f"未找到名为 {name!r} 的 workflow run")

    totals = _download_groups(repo, groups, args)

    if args.dry_run:
        info(f"[dry-run] 将下载 {totals['downloaded']} 条日志，未实际落盘")
        return 0

    summary = (
        f"下载 {totals['downloaded']} / 跳过 {totals['skipped']}"
        f" / 过期 {totals['expired']} / 失败 {totals['failed']} → {OUT_DIR}"
    )
    if totals["failed"]:
        error(f"完成但有失败：{summary}")
        return 1
    success(f"完成：{summary}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (gh.GhError, git.GitError, LogsError) as e:
        error(f"下载 workflow 日志失败: {e}")
        sys.exit(1)
