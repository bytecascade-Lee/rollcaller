#!/usr/bin/env python3
"""
本地更新链路联调 HTTP 服务（模拟远端发布源）。

用法:
    uv run python scripts/serve_local.py <版本号> [--dir <dir>]

服务固定监听 http://localhost:14652（host/port 为常量，与后端 DEVELOP 系列一致），
在 `release/Local` 下查找 `<版本号>+` 前缀的构建产物目录（同版本多个目录取
提交数最多者），只 serve 该目录（模拟"该版本已发布"）。URL 与后端
`common/constant/update.rs` 的 DEVELOP 常量一一对应：

    GET /rollcaller/releases/latest/download/versions.json        → versions_index_url
    GET /rollcaller/releases/latest/download/latest-develop.json  → LATEST_MANIFEST_DEVELOP
    GET /rollcaller/releases/download/v<版本>/latest-develop.json → SPECIFIED_LATEST_MANIFEST_DEVELOP
    GET /rollcaller/releases/download/v<版本>/<资产>               → 清单内产物直链（setup.exe / portable.zip）

目标版本目录不存在、或缺少 latest-develop.json / versions.json 等必要文件时
报错退出并提示先执行本地全链路（release_local.py），不做任何自动生成/修改。
便携版更新器（/updater 前缀，PORTABLE_UPDATER_DEVELOP）暂不 serve。

配套（应用侧）：以后端 UpdateSource::Develop 访问本服务即走本地更新链路。
"""

import argparse
import json
import sys
from pathlib import Path

from common import versions_index
from common.logger import log

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_DIR = versions_index.default_local_dir(ROOT)
# host/port 写死并与 backend common/constant/update.rs DEVELOP 常量一致
HOST = "localhost"
PORT = 14652
# DEVELOP 系列的路径前缀（模拟 github.com/<org>/<repo> 形态）
PREFIX = "/rollcaller"

# serve 目录内必须存在的文件（缺失即报错退出）
REQUIRED_FILES = ("latest-develop.json", "versions.json")


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def validate_dir(serve_dir: Path) -> None:
    """校验 serve 目录包含联调必需文件；缺失报错退出（不做自动生成）。"""
    missing = [name for name in REQUIRED_FILES if not (serve_dir / name).is_file()]
    if missing:
        fail(
            f"目录 {serve_dir} 缺少必要文件: {', '.join(missing)}。\n"
            f"请先用本地全链路生成: uv run python scripts/release_local.py "
            f"{serve_dir.name.split('+', 1)[0]}"
        )
    assets = [
        p.name for p in serve_dir.iterdir()
        if p.is_file() and p.suffix.lower() in (".exe", ".zip")
    ]
    if not assets:
        fail(f"目录 {serve_dir} 下未找到任何安装包/便携版产物（*.exe / *.zip）")
    # 校验索引可解析且为裸数组（防 serve 陈旧/损坏文件）
    try:
        latest = json.loads((serve_dir / "versions.json").read_text(encoding="utf-8"))
        if not isinstance(latest, list) or not latest:
            fail(f"{serve_dir / 'versions.json'} 不是合法的裸数组版本索引")
    except json.JSONDecodeError as e:
        fail(f"{serve_dir / 'versions.json'} 解析失败: {e}")


def build_app(serve_dir: Path, version: str):
    """构造 FastAPI 应用（延迟 import，避免无 fastapi 时脚本 -h 也报错）。"""
    from fastapi import FastAPI, HTTPException
    from fastapi.responses import FileResponse, PlainTextResponse

    app = FastAPI(title="rollcaller local update server", docs_url=None, redoc_url=None)
    v_prefix = f"v{version}"
    latest = f"{PREFIX}/releases/latest/download"
    download = f"{PREFIX}/releases/download"

    def _same_version(req_version: str) -> bool:
        """请求路径中的版本号（可能带前导 v）须与 serve 版本一致。"""
        return req_version.lstrip("vV") == version

    def _serve_file(name: str) -> FileResponse:
        """目录内文件安全 serve（防路径穿越；仅文件，不递归）。"""
        target = (serve_dir / name).resolve()
        if not target.is_relative_to(serve_dir.resolve()) or not target.is_file():
            raise HTTPException(status_code=404, detail="文件不存在")
        return FileResponse(target)

    @app.get("/")
    def index() -> PlainTextResponse:
        assets = sorted(p.name for p in serve_dir.iterdir() if p.is_file())
        return PlainTextResponse(
            "rollcaller local update server\n"
            f"serve version: {version} ({serve_dir})\n"
            "routes:\n"
            f"  {latest}/versions.json\n"
            f"  {latest}/latest-develop.json\n"
            f"  {download}/{v_prefix}/latest-develop.json\n"
            f"  {download}/{v_prefix}/<asset>\n"
            f"assets: {', '.join(assets)}"
        )

    @app.get(f"{latest}/versions.json")
    def versions_index_route() -> FileResponse:
        """VERSIONS_INDEX_DEVELOP：版本索引（每次检查都拉取，不缓存）。"""
        return _serve_file("versions.json")

    @app.get(f"{latest}/latest-develop.json")
    def latest_manifest_route() -> FileResponse:
        """LATEST_MANIFEST_DEVELOP：latest/download 形态（后端当前未消费，serve 同一文件兼容）。"""
        return _serve_file("latest-develop.json")

    @app.get(f"{download}/{{req_version}}/latest-develop.json")
    def specified_manifest_route(req_version: str) -> FileResponse:
        """SPECIFIED_LATEST_MANIFEST_DEVELOP：check.rs 实际消费的目标版本清单。"""
        if not _same_version(req_version):
            raise HTTPException(status_code=404, detail="该版本未在本服务发布")
        return _serve_file("latest-develop.json")

    @app.get(f"{download}/{{req_version}}/{{filename:path}}")
    def download_asset(req_version: str, filename: str) -> FileResponse:
        """清单内产物直链（setup.exe / portable.zip 下载）。"""
        if not _same_version(req_version):
            raise HTTPException(status_code=404, detail="该版本未在本服务发布")
        return _serve_file(filename)

    return app


def main() -> None:
    parser = argparse.ArgumentParser(
        description="本地更新链路联调 HTTP 服务（模拟远端发布源，URL 与后端 DEVELOP 常量同构）"
    )
    parser.add_argument("version", help="要 serve 的核心版本号（如 99.0.0）")
    parser.add_argument("--dir", type=Path, default=DEFAULT_DIR,
                        help=f"产物根目录（默认 {DEFAULT_DIR}）")
    args = parser.parse_args()

    version = args.version.lstrip("vV")
    try:
        serve_dir = versions_index.pick_version_dir(args.dir, version)
    except versions_index.VersionIndexError as e:
        fail(str(e))
    validate_dir(serve_dir)
    log("INFO", f"serve 目录: {serve_dir}")
    log("INFO", f"serve 版本: {version}")
    log("INFO", f"监听: http://{HOST}:{PORT}{PREFIX}/（URL 与后端 DEVELOP 常量同构）")
    log("INFO", "自检: "
                f"curl http://{HOST}:{PORT}{PREFIX}/releases/latest/download/versions.json")

    app = build_app(serve_dir, version)
    import uvicorn
    uvicorn.run(app, host=HOST, port=PORT, log_level="info")


if __name__ == "__main__":
    main()
