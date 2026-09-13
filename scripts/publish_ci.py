#!/usr/bin/env python3
"""
统一发布脚本（CI）：发布 GitHub Release，并把 Release 与自动更新清单同步到 CNB。

取代原 release_ci.py 的 publish 子命令与 sync_cnb.py；清单/版本索引构造与本地发布共用
common.manifest / common.versions_index，GitHub / CNB 命令统一走 common.gh / common.cnb，
两端结构完全一致：
    1. 从 CI 环境提取并校验版本号（复用 build_ci.ci_version）
    2. 从 RELEASE_NOTES.md 提取对应章节作为发布说明（草稿时为占位内容）
    3. 收集构建产物与 .sig 签名（setup/portable 均含），一次生成 latest-github.json 与
       latest-cnb.json（v2 结构，仅附件 URL 指向各自平台的附件直链；severity 取自仓库
       维护的 resources/update/versions.json）。.sig 不发布为附件，仅 base64 嵌进清单
    4. 生成 versions.json 索引附件（版本号 → severity，随双平台 Release 发布，
       客户端据此做坏版本/历史严重级别检测）
    5. 两端显式创建 tag（GitHub refs API / cnb git create-tag）：草稿 Release 不会创建 tag，
       而 CNB 侧 Release 必须挂在 tag 上；且 dispatch 场景下 tag 由本流程现场创建，其 push
       事件来自 GITHUB_TOKEN（不触发 sync-mirrors），只等镜像必然超时
    6. 发布 GitHub Release（gh cli）：2 setup + 2 portable + latest-github.json + versions.json
       - 已存在则改为更新并覆盖附件（重跑幂等）
    7. 发布 CNB Release（cnb cli）：2 setup + 2 portable + latest-cnb.json + versions.json
       - 先给 sync-mirrors 一个等待窗口（tag push 触发时它早已同步完毕），超时则主动创建 tag
       - tag 已有 Release 则更新（patch），否则创建（post）
       - 支持 --draft 与预发布标记（rc 等不置为 latest）

环境变量:
    GITHUB_REPOSITORY / GITHUB_EVENT_NAME / GITHUB_REF_NAME / GITHUB_SHA / INPUT_VERSION
    GH_TOKEN          gh cli 鉴权
    CNB_TOKEN         cnb cli 鉴权（需 repo-code:rw + repo-release:rw）
    CNB_REPO          CNB 仓库路径，默认 ordinary-glory/rollcaller
    DRAFT_RELEASE     为 "true" 时 GitHub/CNB 均发布为草稿

用法:
    uv run python scripts/publish_ci.py [--assets-dir assets]
"""

import argparse
import datetime
import json
import os
import re
import tempfile
import time
from pathlib import Path

from build_ci import ci_version
from common import cnb, gh, manifest, version, versions_index
from common.logger import log

ROOT = Path(__file__).resolve().parent.parent

# 发布为 Release 附件的文件类型（.sig 签名文件不发布，只嵌清单）
ASSET_SUFFIXES = (".exe", ".zip")
# 先给 sync-mirrors 一个把 tag 镜像到 CNB 的等待窗口（秒）：该 workflow 的排队与执行耗时
# 不可控（正常 <1 分钟），窗口内出现即认为镜像链路有效；超时则由本脚本主动创建 tag
MIRROR_WAIT_TIMEOUT = 60
# 主动创建 tag 后的复核窗口（秒）
TAG_CONFIRM_TIMEOUT = 60
# tag 探测的轮询间隔（秒）
TAG_POLL_INTERVAL = 10
# 版本索引源文件（仓库维护，发布期唯一标定 severity 的地方）
VERSIONS_INDEX_PATH = ROOT / "resources" / "update" / "versions.json"


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def github_repo() -> str:
    """取本次发布的目标 GitHub 仓库（owner/repo）。"""
    repo = os.environ.get("GITHUB_REPOSITORY", "")
    if not repo:
        fail("缺少 GITHUB_REPOSITORY 环境变量")
    return repo


def wait_cnb_tag(cnb_repo: str, tag: str, timeout: int) -> bool:
    """在 timeout 秒内轮询 CNB 上的 tag；出现返回 True，超时返回 False（不报错）。"""
    deadline = time.time() + timeout
    while time.time() < deadline:
        if cnb.tag_exists(cnb_repo, tag):
            return True
        time.sleep(TAG_POLL_INTERVAL)
    return False


def ensure_cnb_tag(cnb_repo: str, tag: str, target: str) -> None:
    """确保 CNB 上存在 tag：先等 sync-mirrors，超时后主动创建，最后复核。

    为什么需要主动创建：手动触发（workflow_dispatch）时 tag 由本流程现场创建，该 ref 的
    push 事件由 GITHUB_TOKEN 产生，而 GITHUB_TOKEN 触发的事件不会创建新的 workflow run
    （仅 workflow_dispatch / repository_dispatch 例外），sync-mirrors 不会被唤起，只等不做
    必然超时。tag push 触发时 tag 早已存在，这里自然走"已同步"分支。
    """
    if wait_cnb_tag(cnb_repo, tag, MIRROR_WAIT_TIMEOUT):
        log("INFO", f"CNB 已同步 tag {tag}（sync-mirrors）")
        return
    log("INFO", f"CNB 在 {MIRROR_WAIT_TIMEOUT}s 内未见 tag {tag}，改用 cnb API 创建（target={target[:12]}）")
    try:
        cnb.create_tag(cnb_repo, tag, target)
    except cnb.CnbError as e:
        # 典型情况是并发下 sync-mirrors 恰好把 tag 补上（tag 已存在），交由下面的复核定论
        log("WARN", f"cnb 创建 tag 未成功: {e}")
    if wait_cnb_tag(cnb_repo, tag, TAG_CONFIRM_TIMEOUT):
        log("INFO", f"CNB tag {tag} 已就绪")
        return
    fail(f"CNB 上仍无 tag {tag}（target={target[:12]}）；请确认该提交已镜像到 CNB")


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


def collect_assets(assets_dir: Path, release_version: str) -> tuple[list, dict]:
    """收集安装包/便携版（4 个，发布为附件）与签名（setup/portable 各 2 个，仅嵌清单）。

    Returns:
        files:      待上传附件列表（仅 .exe/.zip，不含 .sig）
        signatures: {arch: {"nsis": setup .sig 全文, "portable": zip .sig 全文}}，
                    缺失或为空均视为构建缺陷，直接报错（非空校验见 manifest.read_sig_text）
    """
    files = sorted(
        p for p in assets_dir.iterdir()
        if p.is_file() and p.suffix.lower() in ASSET_SUFFIXES
    )
    if len(files) != 4:
        fail(
            f"期望 4 个发布产物（2 架构 × 2 文件），实际 {len(files)} 个: "
            f"{[p.name for p in files]}"
        )
    signatures = {}
    for arch in ("x86_64", "arm64"):
        nsis_sigs = sorted(
            assets_dir.glob(f"rollcaller-{release_version}-windows-{arch}-setup.exe.sig")
        )
        if len(nsis_sigs) != 1:
            fail(
                f"缺少 {arch} 的安装包签名 "
                f"rollcaller-{release_version}-windows-{arch}-setup.exe.sig，"
                f"实际 {len(nsis_sigs)} 个: {[p.name for p in nsis_sigs]}"
            )
        portable_sigs = sorted(
            assets_dir.glob(f"rollcaller-{release_version}-windows-{arch}-portable.zip.sig")
        )
        if len(portable_sigs) != 1:
            fail(
                f"缺少 {arch} 的便携版签名 "
                f"rollcaller-{release_version}-windows-{arch}-portable.zip.sig，"
                f"实际 {len(portable_sigs)} 个: {[p.name for p in portable_sigs]}"
            )
        try:
            signatures[arch] = {
                "nsis": manifest.read_sig_text(nsis_sigs[0]),
                "portable": manifest.read_sig_text(portable_sigs[0]),
            }
        except ValueError as e:
            # 非空签名校验收口在 manifest.read_sig_text（与 publish_local 共用）
            fail(str(e))
    return files, signatures


def build_platform_manifests(
    assets_dir: Path,
    release_version: str,
    notes: str,
    signatures: dict,
    make_url,
    severity: str = "normal",
) -> dict:
    """生成 v2 结构 latest.json（与本地 publish_local 产出的 latest-develop.json 同构）。

    setup.exe → 新组 nsis（带 minisign 签名），portable.zip → 新组 portable（zip.sig 签名）；
    signature 直接取 `.sig` 全文（tauri signer ≥2.11 已含 base64(minisign 文本)，不再二次编码）。
    """
    payloads = {}
    for arch in ("x86_64", "arm64"):
        asset = f"rollcaller-{release_version}-windows-{arch}"
        setup = assets_dir / f"{asset}-setup.exe"
        portable = assets_dir / f"{asset}-portable.zip"
        if not setup.is_file():
            fail(f"缺少安装包 {setup.name}，无法生成 {release_version} 的清单")
        sigs = signatures[arch]
        payloads[arch] = {
            "nsis": manifest.build_artifact(make_url(setup.name), setup, sigs["nsis"]),
        }
        if portable.is_file():
            payloads[arch]["portable"] = manifest.build_artifact(
                make_url(portable.name), portable, sigs["portable"]
            )
    return manifest.build_latest_json(
        version=release_version,
        notes=notes,
        severity=severity,
        payloads=payloads,
    )


def ensure_github_tag(gh_repo: str, tag: str, sha: str) -> None:
    """确保 GitHub 上存在 tag（幂等），让草稿与正式发布、GitHub 与 CNB 两端行为一致。

    `gh release create --target <sha>` 会隐式建 tag，但草稿 Release 不会创建 tag，而 CNB 侧
    Release 必须挂在 tag 上，故这里统一显式建 tag。push 触发时 tag 已存在，直接跳过；若已存在
    但指向别的提交则报错，避免把 Release 挂到与 tag 不一致的提交上。
    """
    existing = gh.ref_commit_sha(gh_repo, tag)
    if existing:
        if existing != sha:
            fail(f"GitHub 已存在 tag {tag} 指向 {existing}，与本次发布的提交 {sha} 不一致")
        log("INFO", f"GitHub tag {tag} 已存在（{sha[:12]}）")
        return
    gh.create_tag(gh_repo, tag, sha)
    log("INFO", f"已在 GitHub 创建 tag {tag} → {sha[:12]}")


def publish_github(gh_repo: str, release_version: str, tag: str, notes_path: Path, files: list) -> None:
    """发布 GitHub Release（先），附件含 latest-github.json；已存在则更新（重跑幂等）。"""
    draft = os.environ.get("DRAFT_RELEASE") == "true"
    # tag 已由 ensure_github_tag 就位（create_release 不再需要 --target）；重跑时 Release
    # 已存在，gh release create 必然失败，所以先探测再决定 create 还是 edit + 覆盖上传
    if gh.release_exists(gh_repo, tag):
        gh.edit_release(gh_repo, tag, title=release_version, notes_file=notes_path, draft=draft)
        gh.upload_release_assets(gh_repo, tag, files)
        log("INFO", f"已更新既有 GitHub Release {tag}（附件覆盖上传）")
        return
    gh.create_release(gh_repo, tag, title=release_version, notes_file=notes_path, draft=draft, files=files)


def publish_cnb(release_version: str, tag: str, notes_path: Path, cnb_repo: str, files: list) -> None:
    """发布 CNB Release（后）：tag 已就绪 → 创建/更新 → 上传附件。"""
    draft = os.environ.get("DRAFT_RELEASE") == "true"
    prerelease = version.is_prerelease(release_version)
    make_latest = not (draft or prerelease)

    existing_id = cnb.release_id_by_tag(cnb_repo, tag)
    if existing_id:
        cnb.update_release(
            cnb_repo, existing_id,
            name=release_version,
            body_file=notes_path,
            make_latest=make_latest,
            draft=draft,
            prerelease=prerelease,
        )
        release_id = existing_id
        log("INFO", f"已更新 CNB Release {release_id}（tag {tag}）")
    else:
        # tag 已由 ensure_cnb_tag 就位，Release 直接挂在既有 tag 上（不传 target-commitish，
        # 以免兜底传入的分支名把 tag 挪到分支当前指向）
        log("INFO", f"CNB tag {tag} 暂无 Release，将创建")
        release_id = cnb.create_release(
            cnb_repo, tag,
            name=release_version,
            body_file=notes_path,
            make_latest=make_latest,
            draft=draft,
            prerelease=prerelease,
        )
        log("INFO", f"已创建 CNB Release {release_id}（tag {tag}）")

    for path in files:
        cnb.upload_release_asset(cnb_repo, release_id, path)
    log("INFO", f"CNB Release 同步完成: https://cnb.cool/{cnb_repo}/-/releases/tag/{tag}")


def load_index() -> dict:
    """读取仓库维护的版本索引源文件（resources/update/versions.json）。

    Returns: {版本号: severity}（severity 缺失按 normal 兜底）。源文件缺失视为发布事故。
    """
    try:
        return versions_index.read_entries(VERSIONS_INDEX_PATH)
    except versions_index.VersionIndexError as e:
        fail(str(e))


def main() -> None:
    parser = argparse.ArgumentParser(description="发布 GitHub Release 并同步 CNB Release")
    parser.add_argument("--assets-dir", default="assets", help="构建产物目录（默认 assets）")
    args = parser.parse_args()

    release_version = ci_version()
    tag = f"v{release_version}"
    notes = extract_release_notes(release_version)
    # 写入系统临时目录，避免在 Windows（大小写不敏感文件系统）上
    # release_notes.md 覆盖仓库根目录的 RELEASE_NOTES.md
    fd, notes_path = tempfile.mkstemp(suffix=".md", prefix="release-notes-")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(notes)

        assets_dir = Path(args.assets_dir)
        files, signatures = collect_assets(assets_dir, release_version)

        gh_repo = github_repo()
        cnb_repo = os.environ.get("CNB_REPO", "ordinary-glory/rollcaller")
        # 本次发布的权威提交：dispatch 时是 workflow 选中的分支 head，tag 触发时即 tag 本身；
        # 两端 tag 都以它为目标创建，保证 tag 与 Release 挂在同一提交上
        sha = os.environ.get("GITHUB_SHA", "")
        if not sha:
            fail("缺少 GITHUB_SHA 环境变量")

        # 读取版本索引：标定当前版本的 severity，并生成随 Release 发布的 versions.json
        index = load_index()
        severity = index.get(release_version)
        if severity is None:
            log("WARN", f"versions.json 未标定 {release_version}，本次按 severity=normal 发布；"
                        f"如需标定重要/紧急级别，请先在 {VERSIONS_INDEX_PATH} 中添加")
            severity = "normal"

        # 一次生成两个自动更新清单（同一 v2 模板，附件 URL 指向不同平台）
        latest_github = assets_dir / "latest-github.json"
        latest_github.write_text(
            json.dumps(
                build_platform_manifests(
                    assets_dir, release_version, notes, signatures,
                    lambda asset: f"https://github.com/{gh_repo}/releases/download/{tag}/{asset}",
                    severity,
                ),
                ensure_ascii=False, indent=2,
            ),
            encoding="utf-8",
        )
        log("INFO", f"已生成 {latest_github.name}")
        latest_cnb = assets_dir / "latest-cnb.json"
        latest_cnb.write_text(
            json.dumps(
                build_platform_manifests(
                    assets_dir, release_version, notes, signatures,
                    lambda asset: f"https://cnb.cool/{cnb_repo}/-/releases/download/{tag}/{asset}",
                    severity,
                ),
                ensure_ascii=False, indent=2,
            ),
            encoding="utf-8",
        )
        log("INFO", f"已生成 {latest_cnb.name}")

        # 版本索引附件：两平台同名 versions.json，内容一致（无 URL，仅 版本号 → 严重级别）
        versions_asset = assets_dir / "versions.json"
        entries = versions_index.build_entries({**index, release_version: severity})
        versions_index.write_entries(versions_asset, entries)
        log("INFO", f"已生成 {versions_asset.name}")

        # 1. 两端显式创建 tag（必须在发布 Release 之前：草稿 Release 不会创建 tag）
        ensure_github_tag(gh_repo, tag, sha)

        # 2. GitHub Release（先）
        publish_github(gh_repo, release_version, tag, Path(notes_path), files + [latest_github, versions_asset])

        # 3. 确保 CNB 有 tag：先等 sync-mirrors 镜像，超时则主动创建（dispatch 场景的兜底）
        ensure_cnb_tag(cnb_repo, tag, sha)

        # 4. CNB Release（后）
        publish_cnb(release_version, tag, Path(notes_path), cnb_repo, files + [latest_cnb, versions_asset])
    finally:
        os.unlink(notes_path)


if __name__ == "__main__":
    main()
