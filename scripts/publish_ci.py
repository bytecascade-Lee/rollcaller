#!/usr/bin/env python3
"""
统一发布脚本（CI）：发布 GitHub Release，并把 Release 与自动更新清单同步到 CNB。

取代原 release_ci.py 的 publish 子命令与 sync_cnb.py；清单/版本索引构造与本地发布共用
common.manifest / common.versions_index，两端结构完全一致：
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
import shutil
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

from build_ci import ci_version
from common import manifest, version, versions_index
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


def run_cli(argv: list, label: str, tolerate_failure: bool = False) -> subprocess.CompletedProcess:
    """执行命令行并返回结果；非零退出码默认直接终止。

    tolerate_failure=True 时把非零退出码交给调用方判断（用于"探测类"调用，
    例如查询尚不存在的 Release / git ref，其失败是预期分支而非错误）。
    """
    log("INFO", f"执行: {' '.join(argv)}")
    proc = subprocess.run(
        argv,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    if proc.returncode != 0 and not tolerate_failure:
        fail(f"{label} 失败: {proc.stderr.strip() or proc.stdout.strip()}")
    return proc


def run_cnb(args: list) -> subprocess.CompletedProcess:
    """执行 cnb cli。

    Windows 上 npm 安装的 cnb 是 .cmd shim，无法被 subprocess 直接执行，
    且 cmd.exe /c 会拆解含空格/引号的参数；统一改为 node 直接调用 cli 入口。
    """
    node = shutil.which("node")
    if not node:
        fail("未找到 node（cnb cli 依赖 node，请先安装 Node.js）")
    cnb_shim = shutil.which("cnb")
    if not cnb_shim:
        fail("未找到 cnb 命令（请先安装 @cnbcool/cnb-cli）")
    cli = Path(cnb_shim).resolve().parent / "node_modules" / "@cnbcool" / "cnb-cli" / "bin" / "cnb.js"
    if not cli.exists():
        fail(f"未找到 cnb-cli 入口: {cli}")
    return run_cli([node, str(cli), *args], label=f"cnb {' '.join(args)}")


def cnb_json(args: list) -> object:
    """执行 cnb cli（--verbose）并返回 JSON；自动剥离 {status, data} 包装，错误时终止。"""
    proc = run_cnb([*args, "--verbose"])
    text = proc.stdout.strip()
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        m = re.search(r"\{.*\}", text, re.S)
        if not m:
            fail(f"cnb {' '.join(args)} 输出无法解析为 JSON: {text[:500]}")
        data = json.loads(m.group(0))
    if isinstance(data, dict) and isinstance(data.get("data"), dict) and "errcode" in data["data"]:
        fail(f"cnb {' '.join(args)} 失败: {data['data'].get('errmsg', data['data'])}")
    # 剥离 {status, data: {...}} 包装（成功响应）
    if isinstance(data, dict) and isinstance(data.get("data"), dict):
        return data["data"]
    return data


def cnb_release_id_by_tag(repo: str, tag: str) -> str | None:
    """按 tag 查询 CNB Release；存在返回 id，不存在返回 None。"""
    proc = run_cnb(["releases", "get-release-by-tag", "--repo", repo, "--tag", tag, "--verbose"])
    try:
        data = json.loads(proc.stdout.strip())
    except json.JSONDecodeError:
        return None
    if isinstance(data, dict) and isinstance(data.get("data"), dict) and "errcode" in data["data"]:
        log("INFO", f"CNB tag {tag} 暂无 Release（{data['data'].get('errmsg', '')}），将创建")
        return None
    if isinstance(data, dict) and isinstance(data.get("data"), dict) and "id" in data["data"]:
        return str(data["data"]["id"])
    if isinstance(data, dict) and data.get("id"):
        return str(data["id"])
    fail(f"cnb get-release-by-tag 响应无法识别: {proc.stdout.strip()[:300]}")
    return None


def cnb_api_error(stdout: str) -> str | None:
    """提取 cnb cli 响应里的 API 层错误文案；成功返回 None。

    CNB 的 API 错误不会让进程退出码非 0（实测 get-tag 对不存在的 tag 返回
    errcode=2004002 而 exit code 仍为 0），因此错误只能从响应 JSON 判定。
    """
    text = stdout.strip()
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        m = re.search(r"\{.*\}", text, re.S)
        if not m:
            return f"响应无法解析: {text[:200]}"
        try:
            data = json.loads(m.group(0))
        except json.JSONDecodeError:
            return f"响应无法解析: {text[:200]}"
    inner = data.get("data") if isinstance(data, dict) else None
    if isinstance(inner, dict) and "errcode" in inner:
        return str(inner.get("errmsg") or inner.get("errcode"))
    return None


def cnb_tag_exists(cnb_repo: str, tag: str) -> bool:
    """探测 CNB 上是否已有该 tag。

    走 cnb API 而非 `git ls-remote <内嵌 token 的 URL>`：token 不必进 argv 与错误输出。
    该函数会被轮询调用，故"存在/不存在"都不打日志，异常交由 create-tag 的错误文案暴露。
    """
    proc = run_cnb(["git", "get-tag", "--repo", cnb_repo, "--tag", tag, "--verbose"])
    return cnb_api_error(proc.stdout) is None


def wait_cnb_tag(cnb_repo: str, tag: str, timeout: int) -> bool:
    """在 timeout 秒内轮询 CNB 上的 tag；出现返回 True，超时返回 False（不报错）。"""
    deadline = time.time() + timeout
    while time.time() < deadline:
        if cnb_tag_exists(cnb_repo, tag):
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
    proc = run_cnb([
        "git", "create-tag",
        "--repo", cnb_repo,
        "--name", tag,
        "--target", target,
        "--verbose",
    ])
    error = cnb_api_error(proc.stdout)
    if error:
        # 典型情况是并发下 sync-mirrors 恰好把 tag 补上（tag 已存在），交由下面的复核定论
        log("WARN", f"cnb git create-tag 未成功: {error}")
    if wait_cnb_tag(cnb_repo, tag, TAG_CONFIRM_TIMEOUT):
        log("INFO", f"CNB tag {tag} 已就绪")
        return
    detail = f"create-tag 返回: {error}" if error else "create-tag 未报错"
    fail(f"CNB 上仍无 tag {tag}（{detail}）；请确认目标提交 {target[:12]} 已镜像到 CNB")


def upload_cnb_asset(cnb_repo: str, release_id: str, path: Path) -> None:
    """三步上传附件到 CNB Release：申请预签名 URL → PUT 文件 → 确认上传。"""
    size = path.stat().st_size
    data = cnb_json([
        "releases", "post-release-asset-upload-url",
        "--repo", cnb_repo,
        "--release-id", release_id,
        "--asset-name", path.name,
        "--size", str(size),
        "--overwrite",
        "--ttl", "0",
    ])
    upload_url = data.get("upload_url")
    verify_url = data.get("verify_url")
    if not upload_url or not verify_url:
        fail(f"获取附件上传地址失败: {json.dumps(data, ensure_ascii=False)[:300]}")
    # 预签名 URL：不带鉴权头直接 PUT 文件内容
    req = urllib.request.Request(
        upload_url,
        data=path.read_bytes(),
        method="PUT",
        headers={"Content-Type": "application/octet-stream"},
    )
    try:
        with urllib.request.urlopen(req, timeout=600):
            pass
    except urllib.error.HTTPError as e:
        detail = e.read().decode("utf-8", errors="replace")[:300]
        fail(f"上传附件 {path.name} 失败: HTTP {e.code}: {detail}")
    # 从 verify_url 提取 upload_token / asset_path 并确认
    segments = [urllib.parse.unquote(s) for s in urllib.parse.urlparse(verify_url).path.split("/") if s]
    if len(segments) < 2:
        fail(f"verify_url 无法解析: {verify_url}")
    upload_token, asset_path = segments[-2], segments[-1]
    cnb_json([
        "releases", "post-release-asset-upload-confirmation",
        "--repo", cnb_repo,
        "--release-id", release_id,
        "--upload-token", upload_token,
        "--asset-path", asset_path,
        "--ttl", "0",
    ])
    log("INFO", f"已上传 CNB 附件: {path.name} ({size} bytes)")


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


def ensure_github_tag(tag: str, sha: str) -> None:
    """确保 GitHub 上存在 tag（幂等），让草稿与正式发布、GitHub 与 CNB 两端行为一致。

    `gh release create --target <sha>` 会隐式建 tag，但草稿 Release 不会创建 tag，而 CNB 侧
    Release 必须挂在 tag 上，故这里统一显式建 tag。push 触发时 tag 已存在，直接跳过；若已存在
    但指向别的提交则报错，避免把 Release 挂到与 tag 不一致的提交上。
    """
    repo = os.environ.get("GITHUB_REPOSITORY", "")
    if not repo:
        fail("缺少 GITHUB_REPOSITORY 环境变量")
    # 用 /commits/{ref} 而不是 /git/refs/tags/{tag}：前者对附注 tag 也会解析出真实提交，便于比对
    proc = run_cli(
        ["gh", "api", f"repos/{repo}/commits/{tag}", "--jq", ".sha"],
        label="gh api commits",
        tolerate_failure=True,
    )
    if proc.returncode == 0:
        existing = proc.stdout.strip()
        if existing != sha:
            fail(f"GitHub 已存在 tag {tag} 指向 {existing}，与本次发布的提交 {sha} 不一致")
        log("INFO", f"GitHub tag {tag} 已存在（{sha[:12]}）")
        return
    run_cli(
        ["gh", "api", f"repos/{repo}/git/refs", "-f", f"ref=refs/tags/{tag}", "-f", f"sha={sha}"],
        label="gh api create ref",
    )
    log("INFO", f"已在 GitHub 创建 tag {tag} → {sha[:12]}")


def publish_github(release_version: str, tag: str, notes_path: Path, files: list) -> None:
    """发布 GitHub Release（先），附件含 latest-github.json；已存在则更新（重跑幂等）。"""
    repo = os.environ.get("GITHUB_REPOSITORY", "")
    if not repo:
        fail("缺少 GITHUB_REPOSITORY 环境变量")
    draft = os.environ.get("DRAFT_RELEASE") == "true"
    assets = [str(p) for p in files]
    # tag 已由 ensure_github_tag 就位，故不再需要 --target；重跑时 Release 已存在，
    # 此时 gh release create 必然失败，所以先探测再决定 create 还是 edit + 覆盖上传
    probe = run_cli(
        ["gh", "release", "view", tag, "--repo", repo, "--json", "isDraft"],
        label="gh release view",
        tolerate_failure=True,
    )
    if probe.returncode == 0:
        run_cli([
            "gh", "release", "edit", tag,
            "--repo", repo,
            "--title", release_version,
            "--notes-file", str(notes_path),
            "--draft" if draft else "--draft=false",
        ], label="gh release edit")
        run_cli(
            ["gh", "release", "upload", tag, "--repo", repo, "--clobber", *assets],
            label="gh release upload",
        )
        log("INFO", f"已更新既有 GitHub Release {tag}（附件覆盖上传）")
        return
    args = [
        "release", "create", tag,
        "--repo", repo,
        "--title", release_version,
        "--notes-file", str(notes_path),
    ]
    if draft:
        args.append("--draft")
    run_cli(["gh", *args, *assets], label="gh release create")


def publish_cnb(release_version: str, tag: str, notes_path: Path, cnb_repo: str, files: list) -> None:
    """发布 CNB Release（后）：tag 已就绪 → 创建/更新 → 上传附件。"""
    draft = os.environ.get("DRAFT_RELEASE") == "true"
    prerelease = version.is_prerelease(release_version)
    make_latest = "false" if (draft or prerelease) else "true"

    existing_id = cnb_release_id_by_tag(cnb_repo, tag)
    if existing_id:
        cnb_json([
            "releases", "patch-release",
            "--repo", cnb_repo,
            "--release-id", existing_id,
            "--name", release_version,
            "--body-file", str(notes_path),
            "--make-latest", make_latest,
            *(["--draft"] if draft else []),
            *(["--prerelease"] if prerelease else []),
        ])
        release_id = existing_id
        log("INFO", f"已更新 CNB Release {release_id}（tag {tag}）")
    else:
        # tag 已由 ensure_cnb_tag 就位，Release 直接挂在既有 tag 上；不再传 --target-commitish，
        # 以免兜底传入的分支名把 tag 挪到分支当前指向
        data = cnb_json([
            "releases", "post-release",
            "--repo", cnb_repo,
            "--tag-name", tag,
            "--name", release_version,
            "--body-file", str(notes_path),
            "--make-latest", make_latest,
            *(["--draft"] if draft else []),
            *(["--prerelease"] if prerelease else []),
        ])
        release_id = data.get("id")
        if not release_id:
            fail(f"创建 CNB Release 后未获取到 id: {json.dumps(data, ensure_ascii=False)[:300]}")
        release_id = str(release_id)
        log("INFO", f"已创建 CNB Release {release_id}（tag {tag}）")

    for path in files:
        upload_cnb_asset(cnb_repo, release_id, path)
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

        gh_repo = os.environ.get("GITHUB_REPOSITORY", "")
        if not gh_repo:
            fail("缺少 GITHUB_REPOSITORY 环境变量")
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
        ensure_github_tag(tag, sha)

        # 2. GitHub Release（先）
        publish_github(release_version, tag, Path(notes_path), files + [latest_github, versions_asset])

        # 3. 确保 CNB 有 tag：先等 sync-mirrors 镜像，超时则主动创建（dispatch 场景的兜底）
        ensure_cnb_tag(cnb_repo, tag, sha)

        # 4. CNB Release（后）
        publish_cnb(release_version, tag, Path(notes_path), cnb_repo, files + [latest_cnb, versions_asset])
    finally:
        os.unlink(notes_path)


if __name__ == "__main__":
    main()
