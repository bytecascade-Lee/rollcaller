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
import datetime
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path

from common import builder, packager, targets, tauri_cli, version
from common.logger import log

ROOT = Path(__file__).resolve().parent.parent
BACKEND = ROOT / "backend"


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def ci_version(min_level: str = "rc") -> str:
    """
    从 GitHub Actions 环境变量提取并校验发布版本号。

    min_level: CI 发布默认放行 rc 及以上（禁止 alpha/beta）。
    """
    if os.environ.get("GITHUB_EVENT_NAME") == "workflow_dispatch":
        raw = os.environ.get("INPUT_VERSION")
    else:
        raw = os.environ.get("GITHUB_REF_NAME")
    if not raw:
        raise version.VersionNotFoundError("Unable to obtain version, please check the environment variable settings")
    log("INFO", raw)
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
    log("INFO", f"版本号: {release_version} | arch: {arch} | target: {full_target}")

    # 构建前把 5 个版本文件更新为发布版本，使 tauri.conf.json5 与 tag/input 一致。
    # CI 环境随 job 销毁，无需还原。
    builder.update_version_files(ROOT, release_version)

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
    if os.environ.get("DRAFT_RELEASE") == "true":
        return f"## Draft Release At {datetime.datetime.now()}\n\nComplete the draft here."
    notes_file = ROOT / "RELEASE_NOTES.md"
    if not notes_file.exists():
        fail(f"RELEASE_NOTES.md 不存在: {notes_file}")
    escaped = re.escape(version)
    lines = notes_file.read_text(encoding="utf-8").splitlines()
    in_section = False
    body = []
    for line in lines:
        # 跳过导航锚点注解行（[//]: # (@section: ...) / @link），避免混入发布正文
        if re.match(r"^\s*\[\/\/\]:\s*#\s*\(", line):
            continue
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


def build_latest_json(release_version: str, notes: str, signatures: dict, repo: str) -> dict:
    """按 latest.json.example 模板生成自动更新清单（GitHub 版 latest-github.json）。

    结构严格遵守模板：version / notes / pub_date(RFC 3339) /
    platforms{windows-x86_64, windows-aarch64}，每平台含 signature 与 url。
    signature 取自打包阶段生成的 .sig 文件（base64），url 指向 GitHub Release 附件直链。
    """
    platforms = {}
    for arch, sig in signatures.items():
        asset = f"rollcaller-{release_version}-windows-{arch}-setup.exe"
        platform_key = "windows-aarch64" if arch == "arm64" else f"windows-{arch}"
        platforms[platform_key] = {
            "signature": sig,
            "url": f"https://github.com/{repo}/releases/download/v{release_version}/{asset}",
        }
    pub_date = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    return {
        "version": release_version,
        "notes": notes,
        "pub_date": pub_date,
        "platforms": platforms,
    }


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

        # 收集 .sig 签名（仅用于生成 latest-github.json，不发布为 Release 附件）
        signatures = {}
        for arch in ("x86_64", "arm64"):
            sig_files = sorted(
                assets_dir.glob(f"rollcaller-{release_version}-windows-{arch}-setup.exe.sig")
            )
            if len(sig_files) != 1:
                fail(
                    f"缺少 {arch} 的签名文件 "
                    f"rollcaller-{release_version}-windows-{arch}-setup.exe.sig，"
                    f"实际 {len(sig_files)} 个: {[p.name for p in sig_files]}"
                )
            signatures[arch] = sig_files[0].read_text(encoding="utf-8").strip()

        # 生成自动更新清单并随 Release 发布
        latest_json = build_latest_json(release_version, notes, signatures, repo)
        latest_path = assets_dir / "latest-github.json"
        latest_path.write_text(
            json.dumps(latest_json, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        log("INFO", f"已生成 latest-github.json: {latest_path}")
        files.append(latest_path)

        create_args = [
            "release", "create", tag,
            "--repo", repo,
            "--title", release_version,
            "--notes-file", notes_path,
        ]
        if os.environ.get("DRAFT_RELEASE") == "true":
            create_args.append("--draft")
        # 手动触发时 tag 可能尚不存在，指向本次提交
        if os.environ.get("GITHUB_EVENT_NAME") == "workflow_dispatch":
            sha = os.environ.get("GITHUB_SHA", "")
            create_args += ["--target", sha]
        create_args += [str(p) for p in files]

        log("INFO", f"发布: gh {' '.join(create_args)}")
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
