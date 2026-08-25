#!/usr/bin/env python3
"""
将 GitHub Release 同步到 CNB：创建/更新 CNB Release、上传附件、生成 latest-cnb.json。

用法:
    uv run python scripts/sync_cnb.py --tag v0.1.0 --assets-dir assets [--commitish <sha>]

前置条件:
    - assets 目录中包含 GitHub Release 的全部附件：
      rollcaller-<version>-windows-<arch>-setup.exe ×2、portable.zip ×2、latest-github.json
    - CNB_TOKEN 环境变量：访问令牌，需具备 repo-release:rw 与代码仓库写权限
    - CNB_REPO 环境变量（可选）：CNB 仓库路径，默认 ordinary-glory/rollcaller

流程:
    1. 读取 latest-github.json，按 latest.json.example 模板生成 latest-cnb.json
       （url 指向 CNB Release 附件直链，signature 复用 GitHub 清单中的签名）
    2. 按 tag 查找 CNB Release：存在则 PATCH 更新，不存在则 POST 创建
       （tag 已由 sync-mirrors 工作流同步；target_commitish 仅在 tag 缺失时用于自动打 tag）
    3. 上传 4 个安装包/便携版 + latest-cnb.json 为 Release 附件（.sig 签名文件不发布）

参考: https://docs.cnb.cool/zh/develops/openapi.md（Releases 接口）
"""

import argparse
import json
import os
import urllib.error
import urllib.request
from pathlib import Path

from common import version
from common.logger import log

API_BASE = "https://api.cnb.cool"
# latest.json 平台键 → 产物文件名中的架构标识
ASSET_ARCH_MAP = {"windows-x86_64": "x86_64", "windows-aarch64": "arm64"}
# 发布为 Release 附件的文件类型（.sig 签名文件不发布）
ASSET_SUFFIXES = (".exe", ".zip")


class CnbError(Exception):
    pass


class CnbHttpError(CnbError):
    def __init__(self, status: int, detail: str):
        super().__init__(f"HTTP {status}: {detail}")
        self.status = status


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def _request(method: str, url: str, *, token: str, body: dict | None = None, raw: bytes | None = None) -> object:
    """请求 CNB OpenAPI；url 为完整地址或 API 相对路径。"""
    full_url = url if url.startswith("http") else API_BASE + url
    headers = {
        "Authorization": f"Bearer {token}",
        "Accept": "application/vnd.cnb.api+json",
    }
    data = None
    if body is not None:
        data = json.dumps(body).encode("utf-8")
        headers["Content-Type"] = "application/json"
    elif raw is not None:
        data = raw
        headers["Content-Type"] = "application/octet-stream"
    req = urllib.request.Request(full_url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=600) as resp:
            content = resp.read()
            return json.loads(content.decode("utf-8")) if content else None
    except urllib.error.HTTPError as e:
        detail = e.read().decode("utf-8", errors="replace")[:500]
        raise CnbHttpError(e.code, detail) from e


def upload_asset(token: str, repo: str, release_id: str, path: Path) -> None:
    """三步上传：申请预签名 URL → PUT 文件 → 确认上传。"""
    size = path.stat().st_size
    info = _request(
        "POST",
        f"/{repo}/-/releases/{release_id}/asset-upload-url",
        token=token,
        body={"asset_name": path.name, "size": size, "overwrite": True, "ttl": 0},
    )
    upload_url = info["upload_url"]
    verify_url = info["verify_url"]
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
        raise CnbError(f"上传附件 {path.name} 失败: HTTP {e.code}: {detail}") from e
    # 确认上传（携带鉴权），附件正式关联到 Release
    _request("POST", verify_url, token=token)
    log("INFO", f"已上传附件: {path.name} ({size} bytes)")


def main() -> None:
    parser = argparse.ArgumentParser(description="将 GitHub Release 同步到 CNB")
    parser.add_argument("--tag", required=True, help="GitHub Release 标签，如 v0.1.0")
    parser.add_argument("--assets-dir", default="assets", help="GitHub Release 附件目录")
    parser.add_argument("--commitish", default="", help="Release 对应提交 sha（tag 缺失时用于自动打 tag）")
    args = parser.parse_args()

    token = os.environ.get("CNB_TOKEN")
    if not token:
        fail("缺少 CNB_TOKEN 环境变量（CNB 访问令牌，需 repo-release:rw + 代码仓库写权限）")
    repo = os.environ.get("CNB_REPO", "ordinary-glory/rollcaller")
    tag = args.tag
    assets = Path(args.assets_dir)

    # 1. 读取 GitHub 清单，生成 latest-cnb.json（严格遵循 latest.json.example 模板）
    latest_github = assets / "latest-github.json"
    if not latest_github.exists():
        fail(f"缺少 {latest_github}（请确认 GitHub Release 已发布并包含该附件）")
    meta = json.loads(latest_github.read_text(encoding="utf-8"))
    release_version = meta["version"]
    notes = meta["notes"]

    platforms = {}
    for plat, arch in ASSET_ARCH_MAP.items():
        entry = meta["platforms"].get(plat)
        if not entry or not entry.get("signature"):
            fail(f"latest-github.json 缺少平台 {plat} 的签名")
        asset_name = f"rollcaller-{release_version}-windows-{arch}-setup.exe"
        platforms[plat] = {
            "signature": entry["signature"],
            "url": f"https://cnb.cool/{repo}/-/releases/download/{tag}/{asset_name}",
        }
    latest_cnb = {
        "version": release_version,
        "notes": notes,
        "pub_date": meta["pub_date"],
        "platforms": platforms,
    }
    latest_cnb_path = assets / "latest-cnb.json"
    latest_cnb_path.write_text(
        json.dumps(latest_cnb, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    log("INFO", f"已生成 latest-cnb.json: {latest_cnb_path}")

    # 2. 创建/更新 CNB Release（预发布不置为 latest，与 GitHub 语义一致）
    prerelease = version.is_prerelease(release_version)
    form = {
        "name": release_version,
        "body": notes,
        "prerelease": prerelease,
        "make_latest": "false" if prerelease else "true",
    }
    try:
        existing = _request("GET", f"/{repo}/-/releases/tags/{tag}", token=token)
    except CnbHttpError as e:
        if e.status != 404:
            raise
        existing = None
    if existing:
        release_id = existing["id"]
        _request("PATCH", f"/{repo}/-/releases/{release_id}", token=token, body=form)
        log("INFO", f"已更新 CNB Release {release_id}（tag {tag}）")
    else:
        form.update({"tag_name": tag, "target_commitish": args.commitish or tag})
        created = _request("POST", f"/{repo}/-/releases", token=token, body=form)
        release_id = created["id"]
        log("INFO", f"已创建 CNB Release {release_id}（tag {tag}）")

    # 3. 上传附件（.sig 不发布）
    upload_files = sorted(
        p for p in assets.iterdir()
        if p.is_file() and p.suffix.lower() in ASSET_SUFFIXES
    ) + [latest_cnb_path]
    if len(upload_files) != 5:
        fail(
            f"期望上传 5 个附件（2 架构 × 2 文件 + latest-cnb.json），"
            f"实际 {len(upload_files)} 个: {[p.name for p in upload_files]}"
        )
    for path in upload_files:
        upload_asset(token, repo, release_id, path)

    log("INFO", f"CNB Release 同步完成: https://cnb.cool/{repo}/-/releases/tag/{tag}")


if __name__ == "__main__":
    main()
