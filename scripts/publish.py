#!/usr/bin/env python3
"""
统一发布脚本：发布 GitHub Release，并把 Release 与自动更新清单同步到 CNB。

取代原 release_ci.py 的 publish 子命令与 sync_cnb.py：
    1. 从 CI 环境提取并校验版本号（复用 release_ci.ci_version）
    2. 从 RELEASE_NOTES.md 提取对应章节作为发布说明（草稿时为占位内容）
    3. 收集构建产物 .sig 签名，一次生成 latest-github.json 与 latest-cnb.json
       （同一份 latest.json 模板，仅附件 URL 指向各自平台的附件直链；
       severity/force 取自仓库维护的 resources/update/versions.json）
    4. 生成 versions.json 索引附件（版本号 → severity/force，随双平台 Release 发布，
       客户端据此做坏版本/历史严重级别检测）
    5. 发布 GitHub Release（gh cli）：2 setup + 2 portable + latest-github.json + versions.json
    6. 发布 CNB Release（cnb cli）：2 setup + 2 portable + latest-cnb.json + versions.json
       - 先等待 sync-mirrors 把 tag 同步到 CNB（dispatch 触发的 tag 需时间推送）
       - tag 已有 Release 则更新（patch），否则创建（post）
       - 支持 --draft 与预发布标记（rc 等不置为 latest）

环境变量:
    GITHUB_REPOSITORY / GITHUB_EVENT_NAME / GITHUB_REF_NAME / GITHUB_SHA / INPUT_VERSION
    GH_TOKEN          gh cli 鉴权
    CNB_TOKEN         cnb cli 鉴权（需 repo-code:rw + repo-release:rw）
    CNB_REPO          CNB 仓库路径，默认 ordinary-glory/rollcaller
    DRAFT_RELEASE     为 "true" 时 GitHub/CNB 均发布为草稿

用法:
    uv run python scripts/publish.py [--assets-dir assets]
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

from common import version
from common.logger import log
from release_ci import ci_version

ROOT = Path(__file__).resolve().parent.parent

# latest.json 平台键 → 产物文件名中的架构标识
ASSET_ARCH_MAP = {"windows-x86_64": "x86_64", "windows-aarch64": "arm64"}
# 发布为 Release 附件的文件类型（.sig 签名文件不发布）
ASSET_SUFFIXES = (".exe", ".zip")
# 等待 sync-mirrors 把 tag 同步到 CNB 的超时（秒）
CNB_TAG_SYNC_TIMEOUT = 300
# severity 合法档位（与后端 manifest.rs 的 Severity 枚举一致）
SEVERITY_LEVELS = ("normal", "important", "critical")
# 版本索引源文件（仓库维护，发布期唯一标定 severity/force 的地方）
VERSIONS_INDEX_PATH = ROOT / "resources" / "update" / "versions.json"


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def run_cli(argv: list, label: str) -> subprocess.CompletedProcess:
    """执行命令行并返回结果；非零退出码直接终止。"""
    log("INFO", f"执行: {' '.join(argv)}")
    proc = subprocess.run(
        argv,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    if proc.returncode != 0:
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


def wait_for_cnb_tag(cnb_repo: str, tag: str, timeout: int = CNB_TAG_SYNC_TIMEOUT) -> None:
    """等待 sync-mirrors 把 tag 推送到 CNB（dispatch 手动触发的 tag 需要时间同步）。"""
    token = os.environ.get("CNB_TOKEN", "")
    url = f"https://cnb:{token}@cnb.cool/{cnb_repo}.git"
    deadline = time.time() + timeout
    while time.time() < deadline:
        proc = subprocess.run(
            ["git", "ls-remote", url, f"refs/tags/{tag}"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
        if proc.returncode == 0 and proc.stdout.strip():
            log("INFO", f"CNB 已同步 tag {tag}")
            return
        time.sleep(10)
    fail(f"tag {tag} 未在 {timeout} 秒内同步到 CNB（请检查 sync-mirrors 工作流）")


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


def load_versions_index() -> dict:
    """读取仓库维护的版本索引（resources/update/versions.json）。

    索引是发布期唯一标定 severity/force 的地方：发布时用当前版本条目丰富 latest-*.json，
    并把整个索引作为 versions.json 附件随 Release 发布（客户端据此做坏版本/历史严重级别检测）。

    Returns:
        {版本号: {"severity": str, "force": bool}}，版本号已规范化（去除前导 v）
    """
    if not VERSIONS_INDEX_PATH.exists():
        fail(f"缺少版本索引源文件: {VERSIONS_INDEX_PATH}（发布前应维护，标注各版本 severity/force）")
    try:
        data = json.loads(VERSIONS_INDEX_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        fail(f"versions.json 解析失败: {e}")
    entries = data.get("versions")
    if not isinstance(entries, list):
        fail("versions.json 缺少 versions 数组")
    index = {}
    for item in entries:
        raw = item.get("version")
        if not raw:
            fail("versions.json 中存在缺少 version 的条目")
        ver = version.validate(raw)
        severity = item.get("severity", "normal")
        if severity not in SEVERITY_LEVELS:
            fail(f"版本 {raw} 的 severity={severity!r} 非法（应为 normal/important/critical）")
        index[ver] = {"severity": severity, "force": bool(item.get("force", False))}
    return index


def version_severity(index: dict, release_version: str) -> tuple:
    """当前发布版本的 (severity, force)；未在索引中标定时按 normal/false 兜底并告警。"""
    entry = index.get(release_version)
    if entry is None:
        log("WARN", f"versions.json 未标定 {release_version}，本次按 severity=normal 发布；"
                    f"如需标定重要/紧急级别，请先在 {VERSIONS_INDEX_PATH} 中添加")
        return "normal", False
    return entry["severity"], entry["force"]


def build_versions_asset(index: dict, release_version: str, severity: str, force: bool) -> dict:
    """构建随 Release 发布的 versions.json：源索引 + 当前版本兜底（normal），按版本号倒序。

    索引内容不含任何 URL——客户端凭版本号即可拼接出对应 Release 的 latest-*.json 地址。
    """
    versions = [
        {"version": ver, "severity": entry["severity"], "force": entry["force"]}
        for ver, entry in index.items()
    ]
    if release_version not in index:
        versions.append({"version": release_version, "severity": severity, "force": force})
    versions.sort(key=lambda x: version.parse(x["version"])[:3], reverse=True)
    return {"versions": versions}


def build_latest_json(release_version: str, notes: str, signatures: dict, make_url,
                      severity: str = "normal", force: bool = False) -> dict:
    """生成自动更新清单。

    同一结构同时用于 latest-github.json 与 latest-cnb.json，仅附件 URL 不同：
    make_url(asset_name) 返回该平台下的附件直链。
    severity/force 来自仓库维护的 versions.json 索引，随清单下发给客户端。
    """
    platforms = {}
    for arch, sig in signatures.items():
        asset = f"rollcaller-{release_version}-windows-{arch}-setup.exe"
        platform_key = "windows-aarch64" if arch == "arm64" else f"windows-{arch}"
        platforms[platform_key] = {
            "signature": sig,
            "url": make_url(asset),
        }
    pub_date = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    return {
        "version": release_version,
        "notes": notes,
        "pub_date": pub_date,
        "severity": severity,
        "force": force,
        "platforms": platforms,
    }


def collect_assets(assets_dir: Path, release_version: str) -> tuple[list, dict]:
    """收集安装包/便携版（4 个）与 .sig 签名（2 个，仅用于生成 latest.json，不发布）。"""
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
    return files, signatures


def github_release_commitish(tag: str) -> str:
    """获取 GitHub Release 对应的 targetCommitish（CNB 创建 Release 的兜底打 tag 目标）。"""
    repo = os.environ.get("GITHUB_REPOSITORY", "")
    proc = run_cli(
        ["gh", "release", "view", tag, "--repo", repo, "--json", "targetCommitish", "--jq", ".targetCommitish"],
        label="gh release view",
    )
    commitish = proc.stdout.strip()
    if not commitish:
        fail(f"gh release view {tag} 未返回 targetCommitish")
    return commitish


def publish_github(release_version: str, tag: str, notes_path: Path, files: list) -> None:
    """发布 GitHub Release（先），附件含 latest-github.json。"""
    repo = os.environ.get("GITHUB_REPOSITORY", "")
    if not repo:
        fail("缺少 GITHUB_REPOSITORY 环境变量")
    args = [
        "release", "create", tag,
        "--repo", repo,
        "--title", release_version,
        "--notes-file", str(notes_path),
    ]
    if os.environ.get("DRAFT_RELEASE") == "true":
        args.append("--draft")
    # 手动触发时 tag 可能尚不存在，指向本次提交
    if os.environ.get("GITHUB_EVENT_NAME") == "workflow_dispatch":
        sha = os.environ.get("GITHUB_SHA", "")
        args += ["--target", sha]
    args += [str(p) for p in files]
    run_cli(["gh", *args], label="gh release create")


def publish_cnb(release_version: str, tag: str, notes_path: Path, cnb_repo: str, files: list) -> None:
    """发布 CNB Release（后）：等待 tag 同步 → 创建/更新 → 上传附件。"""
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
        commitish = github_release_commitish(tag)
        data = cnb_json([
            "releases", "post-release",
            "--repo", cnb_repo,
            "--tag-name", tag,
            "--target-commitish", commitish,
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

        # 读取版本索引：标定当前版本的 severity/force，并生成随 Release 发布的 versions.json
        index = load_versions_index()
        severity, force = version_severity(index, release_version)

        # 一次生成两个自动更新清单（同一模板，附件 URL 指向不同平台）
        latest_github = assets_dir / "latest-github.json"
        latest_github.write_text(
            json.dumps(
                build_latest_json(
                    release_version, notes, signatures,
                    lambda asset: f"https://github.com/{gh_repo}/releases/download/{tag}/{asset}",
                    severity, force,
                ),
                ensure_ascii=False, indent=2,
            ),
            encoding="utf-8",
        )
        log("INFO", f"已生成 {latest_github.name}")
        latest_cnb = assets_dir / "latest-cnb.json"
        latest_cnb.write_text(
            json.dumps(
                build_latest_json(
                    release_version, notes, signatures,
                    lambda asset: f"https://cnb.cool/{cnb_repo}/-/releases/download/{tag}/{asset}",
                    severity, force,
                ),
                ensure_ascii=False, indent=2,
            ),
            encoding="utf-8",
        )
        log("INFO", f"已生成 {latest_cnb.name}")

        # 版本索引附件：两平台同名 versions.json，内容一致（无 URL，仅 版本号 → 严重级别）
        versions_asset = assets_dir / "versions.json"
        versions_asset.write_text(
            json.dumps(
                build_versions_asset(index, release_version, severity, force),
                ensure_ascii=False, indent=2,
            ),
            encoding="utf-8",
        )
        log("INFO", f"已生成 {versions_asset.name}")

        # 1. GitHub Release（先）
        publish_github(release_version, tag, Path(notes_path), files + [latest_github, versions_asset])

        # 2. CNB Release（后）：dispatch 触发的 tag 需等待 sync-mirrors 推送完成
        wait_for_cnb_tag(cnb_repo, tag)
        publish_cnb(release_version, tag, Path(notes_path), cnb_repo, files + [latest_cnb, versions_asset])
    finally:
        os.unlink(notes_path)


if __name__ == "__main__":
    main()
