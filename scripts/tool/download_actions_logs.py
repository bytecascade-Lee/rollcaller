#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
下载 GitHub Actions workflow run 的运行日志（zip），按 workflow 分目录归档。

目录结构：
    release/CI-CD/logs/
    ├── Release/
    │   ├── Release#4-34004489264.zip
    │   └── Release#3-33312690776.zip
    └── Sync Mirrors/
        └── Sync Mirrors#13-34004489242.zip

命名规则：
    <workflow 显示名>#<run 序号>-<run 数据库 ID>.zip
    - run 序号：该 workflow 内递增的 run_number（对应 Actions 页 Release #N）
    - run 数据库 ID：下载日志使用的编号，全局唯一

特性：
    - 幂等：目标 zip 已存在则跳过，增量同步，--force 强制重下
    - 容错：单条失败记 WARNING 继续，结尾汇总各 workflow 的成功 / 跳过 / 失败数
    - 预检 gh 登录，未登录给出明确指引
    - 默认只处理 .github/workflows 下真实 workflow 的 run；
      GitHub 内部自动 workflow（如 Dependency Graph 依赖更新）默认跳过，--include-gh-runs 可放开

前置条件：
    - 已安装并登录 GitHub CLI（gh auth login，或导出 GH_TOKEN）
    - 目标仓库按 owner/repo 形式解析：GITHUB_REPOSITORY 环境变量
      → git remote(github → origin) 顺序

用法:
    uv run python scripts/download_actions_logs.py
    uv run python scripts/download_actions_logs.py --workflow Release
    uv run python scripts/download_actions_logs.py --since 2026-09-01 --dry-run
    uv run python scripts/download_actions_logs.py --include-running --force
"""

import argparse
import os
import re
import sys
import time
from collections import OrderedDict
from typing import Dict, List, Optional

import util
from cli import git, gh
from common.constant.paths import PROJECT_BASE_DIR, RELEASE_DIR
from common.logger import error, info, success, warning

OUT_DIR = RELEASE_DIR / "CI-CD" / "logs"

# 解析 git remote URL 中的 owner/repo（支持 https 与 git@github.com: 两种形式）
_REPO_RE = re.compile(r"github\.com[:/]([^/:]+)/([^/.]+?)(?:\.git)?/?$")


def repo_from_url(url: str) -> Optional[str]:
    """从 git remote URL 提取 owner/repo；非 GitHub 地址返回 None"""
    match = _REPO_RE.search(url)
    return f"{match.group(1)}/{match.group(2)}" if match else None


def resolve_repo() -> str:
    """解析目标仓库：GITHUB_REPOSITORY > git remote(github → origin)"""
    env_repo = os.environ.get("GITHUB_REPOSITORY")
    if env_repo:
        return env_repo

    for remote in ("github", "origin"):
        try:
            url = git.git(["remote", "get-url", remote], cwd=PROJECT_BASE_DIR)
        except git.GitError:
            continue
        repo = repo_from_url(url)
        if repo:
            info(f"从 git remote {remote} 解析到仓库: {repo}")
            return repo

    util.fail(
        "无法确定目标仓库。请设置 GITHUB_REPOSITORY 环境变量，"
        "或为本仓库配置 github / origin 远端。"
    )


def sanitize_dir_name(name: str) -> str:
    """workflow 显示名用作目录名；替换 Windows 路径非法字符"""
    cleaned = re.sub(r'[<>:"/\\|?*\x00-\x1f]', "_", name).strip().rstrip(". ")
    return cleaned or "unknown-workflow"


def should_include_run(run: Dict, args: argparse.Namespace) -> bool:
    """按 --workflow / 状态 / --since 过滤单个 run"""
    if args.workflow and run.get("wf_display") not in args.workflow:
        return False
    if not args.include_running and run.get("status") != "completed":
        return False
    if args.since:
        created = run.get("created_at") or ""
        if created[:10] < args.since:
            return False
    return True


def group_runs(runs: List[Dict]) -> "OrderedDict[str, List[Dict]]":
    """按 workflow 显示名（wf_display）分组，组内按 run 序号升序"""
    groups: "OrderedDict[str, List[Dict]]" = OrderedDict()
    for run in runs:
        name = run.get("wf_display") or "unknown"
        groups.setdefault(name, []).append(run)
    for items in groups.values():
        items.sort(key=lambda r: r.get("run_number") or 0)
    return groups


def main() -> None:
    parser = argparse.ArgumentParser(
        description="下载 GitHub Actions workflow run 日志并归档到 release/CI-CD/logs",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "示例:\n"
            "  uv run python scripts/download_actions_logs.py\n"
            "  uv run python scripts/download_actions_logs.py --workflow Release\n"
            "  uv run python scripts/download_actions_logs.py --since 2026-09-01 --dry-run\n"
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

    args = parser.parse_args()

    repo = resolve_repo()
    info(f"目标仓库: {repo}")
    info(f"输出目录: {OUT_DIR}")

    # 枚举 run 列表本身需要网络与鉴权，预检无条件执行
    try:
        gh.require_auth()
    except gh.GhAuthError as e:
        util.fail(str(e))
    info("gh 鉴权预检通过")

    # 枚举全部 run（最新在前）
    info("正在枚举 workflow runs ...")
    runs = gh.list_runs(repo, limit=args.limit)

    # workflow 显示名与归属以 workflows API 为准，但该 API 同样包含 GitHub 内部
    # 自动 workflow（如 Dependency Graph，run 的 name 是 "Graph Update: ..." 这类
    # 更新标题而非 workflow 名）。因此用 .github/workflows 目录的真实文件做白名单，
    # 只有真实文件的 run 才处理；内部自动 workflow 默认跳过（--include-gh-runs 放开）。
    workflows = gh.list_workflows(repo)
    wf_name_by_id = {w["id"]: w["name"] for w in workflows}
    if not wf_name_by_id:
        warning("workflows API 未返回任何 workflow，退化为按 run 自身字段分组")

    try:
        wf_files = set(gh.list_dir_names(repo, ".github/workflows"))
    except gh.GhError as e:
        warning(f"读取 .github/workflows 目录失败，本次不过滤内部 workflow: {e}")
        wf_files = None
    if wf_files:
        real_wf_ids = {w["id"] for w in workflows if w.get("path") in {f".github/workflows/{f}" for f in wf_files}}
    else:
        real_wf_ids = set(wf_name_by_id)
        if wf_files is not None:
            warning(".github/workflows 目录为空或无交集，本次不过滤内部 workflow")
    if wf_files and not real_wf_ids:
        warning("workflows API 路径与 .github/workflows 文件无交集，退化为不过滤")
        real_wf_ids = set(wf_name_by_id)

    kept: List[Dict] = []
    skipped_internal = 0
    for run in runs:
        wf_id = run.get("workflow_id")
        run["wf_display"] = wf_name_by_id.get(wf_id) or run.get("name") or "unknown"
        if not args.include_gh_runs and wf_id not in real_wf_ids:
            skipped_internal += 1
            continue
        kept.append(run)
    runs = kept
    if skipped_internal:
        info(f"跳过 {skipped_internal} 条 GitHub 内部自动 workflow 的 run"
             f"（如 Dependency Graph，可用 --include-gh-runs 放开）")

    if args.workflow:
        info(f"已过滤 workflow: {', '.join(args.workflow)}")
    if args.since:
        info(f"已过滤起始日期: {args.since}")
    if not args.include_running:
        skipped_running = sum(1 for r in runs if r.get("status") != "completed")
        if skipped_running:
            info(f"跳过 {skipped_running} 条未完成的 run（可用 --include-running 包含）")

    selected = [r for r in runs if should_include_run(r, args)]
    groups = group_runs(selected)
    if not groups:
        util.fail("没有符合条件（含日期/状态/workflow 过滤）的 run")

    info(f"共 {len(selected)} 条 run 待处理，按 {len(groups)} 个 workflow 分组")
    if args.workflow:
        found = {g for g in groups}
        missing = sorted(set(args.workflow) - found)
        for name in missing:
            warning(f"未找到名为 {name!r} 的 workflow run")

    totals = {"downloaded": 0, "skipped": 0, "failed": 0, "expired": 0}
    for wf_name, wf_runs in groups.items():
        # 目录与文件名前缀统一用 sanitize 后的安全名，避免 `:` `/` 等
        # Windows 非法字符导致 mkdir / 写文件失败
        safe = sanitize_dir_name(wf_name)
        wf_dir = OUT_DIR / safe
        stats = {"downloaded": 0, "skipped": 0, "failed": 0, "expired": 0}
        info(f"--- workflow: {wf_name}（{len(wf_runs)} 条）---")

        for run in wf_runs:
            run_id = run.get("id")
            number = run.get("run_number")
            if run_id is None or number is None:
                stats["failed"] += 1
                error(f"{wf_name}: run 缺 id/run_number 字段，跳过（{run}）")
                continue
            fname = f"{safe}#{number}-{run_id}.zip"
            dest = wf_dir / fname
            verb = "跳过（已存在）" if dest.exists() and not args.force else "下载"
            info(f"[{verb}] {dest}")

            if dest.exists() and not args.force:
                stats["skipped"] += 1
                continue
            if args.dry_run:
                stats["downloaded"] += 1
                continue

            try:
                gh.download_run_log(repo, run_id, dest)
                stats["downloaded"] += 1
            except Exception as e:  # gh.GhError / OSError 等：单条失败绝不中断批次
                if isinstance(e, gh.GhError):
                    code = gh.http_status(e)
                    if code in (404, 410):
                        stats["expired"] += 1
                        warning(f"{fname}: 日志不可用（HTTP {code}，可能超过保留期或尚未生成）")
                        continue
                stats["failed"] += 1
                error(f"{fname}: 下载失败: {e}")
                continue

            if args.interval > 0 and not args.dry_run:
                time.sleep(args.interval)

        info(f"--- {wf_name}: 下载 {stats['downloaded']} / 跳过 {stats['skipped']}"
             f" / 过期 {stats['expired']} / 失败 {stats['failed']} ---")
        for key in totals:
            totals[key] += stats[key]

    if args.dry_run:
        info(f"[dry-run] 将下载 {totals['downloaded']} 条日志，未实际落盘")
    else:
        success(f"完成：下载 {totals['downloaded']} / 跳过 {totals['skipped']}"
                f" / 过期 {totals['expired']} / 失败 {totals['failed']}"
                f" → {OUT_DIR}")
        if totals["failed"]:
            sys.exit(2)


if __name__ == "__main__":
    main()
