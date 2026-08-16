#!/usr/bin/env python3
"""
CI 发布脚本：构建/打包（build）与发布 GitHub Release（publish）。

版本号来源（GitHub Actions 环境变量，由脚本统一提取与校验，而非在 workflow 中
用 PowerShell 重复实现）：
    - workflow_dispatch: 从 inputs.version 读取（INPUT_VERSION）
    - push tag: 从 GITHUB_REF_NAME 读取（自动去掉前导 v）
等级校验（min_level="rc"）：放行 rc 及以上，禁止 alpha/beta。

用法:
    uv run python scripts/release_ci.py build --target <target>
    uv run python scripts/release_ci.py publish
"""

import argparse
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

from common import builder, packager, targets, tauri_cli, version

ROOT = Path(__file__).resolve().parent.parent
BACKEND = ROOT / "backend"


def fail(message: str) -> None:
    print(f"[错误] {message}", file=sys.stderr)
    raise SystemExit(1)


def ci_version(min_level: str = "rc") -> str:
    """
    从 GitHub Actions 环境变量提取并校验发布版本号。

    min_level: CI 发布默认放行 rc 及以上（禁止 alpha/beta）。
    """
    if os.environ.get("GITHUB_EVENT_NAME") == "workflow_dispatch":
        raw = os.environ.get("INPUT_VERSION", "")
    else:
        raw = os.environ.get("GITHUB_REF_NAME", "")
    try:
        return version.validate(raw, min_level=min_level)
    except version.VersionError as e:
        fail(str(e))


def cmd_build(target: str) -> None:
    if not target:
        fail("build 需要 --target（架构，支持别名或完整三元组）")
    try:
        full_target = targets.resolve_target(target)
    except targets.TargetError as e:
        fail(str(e))
    release_version = ci_version()
    arch = packager.arch_for_target(full_target)
    print(f">> 版本号: {release_version} | arch: {arch} | target: {full_target}")

    cli_label, cli_cmd = tauri_cli.resolve(ROOT)
    release_dir = builder.build(
        ROOT,
        BACKEND,
        full_target,
        cli_cmd,
        cli_label,
        # BRANCH_NAME/VERSION 被 backend/build.rs 读取并嵌入二进制
        env_overrides={"VERSION": release_version, "BRANCH_NAME": "master"},
    )
    # CI 产物直接输出到工作区根目录，供 upload-artifact 收集
    packager.package_setup(release_dir, release_version, arch, ROOT)
    packager.package_portable(release_dir, release_version, arch, ROOT)


def extract_release_notes(version: str) -> str:
    """从 RELEASE_NOTES.md 提取 '## <version>' 章节。"""
    notes_file = ROOT / "RELEASE_NOTES.md"
    if not notes_file.exists():
        fail(f"RELEASE_NOTES.md 不存在: {notes_file}")
    escaped = re.escape(version)
    lines = notes_file.read_text(encoding="utf-8").splitlines()
    in_section = False
    body = []
    for line in lines:
        if re.match(rf"^## {escaped}\s*$", line):
            in_section = True
            continue
        if in_section and re.match(r"^##\s", line):
            break
        if in_section:
            body.append(line)
    if not body:
        fail(f"RELEASE_NOTES.md 中未找到 '## {version}' 章节")
    return "\n".join(body).rstrip() + "\n"


def cmd_publish() -> None:
    release_version = ci_version()
    tag = f"v{release_version}"
    notes = extract_release_notes(release_version)
    # 写入系统临时目录，避免在 Windows（大小写不敏感文件系统）上
    # release_notes.md 覆盖仓库根目录的 RELEASE_NOTES.md
    fd, notes_path = tempfile.mkstemp(suffix=".md", prefix="release-notes-")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(notes)

        assets_dir = ROOT / "assets"
        files = sorted(
            p
            for p in assets_dir.iterdir()
            if p.is_file() and p.suffix.lower() in (".exe", ".zip")
        )
        if len(files) != 4:
            fail(
                f"期望 4 个发布产物（2 架构 × 2 文件），实际 {len(files)} 个: "
                f"{[p.name for p in files]}"
            )

        repo = os.environ.get("GITHUB_REPOSITORY", "")
        if not repo:
            fail("缺少 GITHUB_REPOSITORY 环境变量")

        create_args = [
            "release", "create", tag,
            "--repo", repo,
            "--title", release_version,
            "--notes-file", notes_path,
        ]
        # 手动触发时 tag 可能尚不存在，指向本次提交
        if os.environ.get("GITHUB_EVENT_NAME") == "workflow_dispatch":
            sha = os.environ.get("GITHUB_SHA", "")
            create_args += ["--target", sha]
        create_args += [str(p) for p in files]

        print(f">> 发布: gh {' '.join(create_args)}")
        result = subprocess.run(
            ["gh", *create_args],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
        if result.returncode != 0:
            fail(f"创建 release {tag} 失败: {result.stderr.strip()}")
    finally:
        os.unlink(notes_path)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="CI 发布脚本：构建打包（build）与发布 GitHub Release（publish）"
    )
    sub = parser.add_subparsers(dest="command", required=True)
    build_p = sub.add_parser("build", help="构建并打包（matrix 中每个架构各跑一次）")
    build_p.add_argument(
        "--target", required=True,
        help="架构：完整三元组或别名，如 x86_64-pc-windows-msvc / arm64",
    )
    sub.add_parser("publish", help="汇总产物并发布 GitHub Release")

    args = parser.parse_args()
    if args.command == "build":
        cmd_build(args.target)
    else:
        cmd_publish()


if __name__ == "__main__":
    main()
