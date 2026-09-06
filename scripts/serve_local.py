#!/usr/bin/env python3
"""
本地更新链路联调 HTTP 服务（模拟远端发布源）。

用法:
    uv run python scripts/serve_local.py --version 99.0.0 [--port 14652] [--dir release/Local]

服务在 `release/Local` 下查找 `--version`（如 99.0.0）对应的构建产物目录
（目录名 `<版本>+<分支>.<提交数>.<短哈希>`；同版本多个目录取提交数最多者），
只 serve 该目录（模拟"该版本已发布"），URL 形状与 GitHub/CNB Release 资产同构：

    GET /releases/latest/download/versions.json    → 该目录 versions.json
    GET /releases/download/v<ver>/latest-dev.json  → 该目录 latest-dev.json
    GET /releases/download/v<ver>/<任意资产>        → setup.exe / portable.zip 等

若目标版本目录不存在、或缺少 latest-dev.json / versions.json 等必要文件，
报错退出并提示先执行本地构建（release_local.py），不做任何自动生成/修改。

配套（应用侧启动）:
    ROLLCALLER_UPDATE_BASE=http://127.0.0.1:14652 cargo tauri dev
"""

import argparse
import json
import sys
from pathlib import Path

from common.logger import log

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_DIR = ROOT / "release" / "Local"
DEFAULT_PORT = 14652

# serve 目录内必须存在的文件（缺失即报错退出）
REQUIRED_FILES = ("latest-dev.json", "versions.json")


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def parse_build_count(dir_name: str) -> int:
    """从 `<core>+<branch>.<count>.<hash>` 中解析提交数（build 倒数第 2 段）。

    分支名可含 '.'，因此不能按固定下标取；build 段末尾恒为 `<count>.<short_hash>`。
    """
    build = dir_name.split("+", 1)[1]
    parts = build.split(".")
    try:
        return int(parts[-2])
    except (IndexError, ValueError):
        return -1


def pick_version_dir(local_dir: Path, version: str) -> Path:
    """在 release/Local 中挑选 `version+` 前缀的产物目录（多者取提交数最多）。"""
    if not local_dir.is_dir():
        fail(f"产物根目录不存在: {local_dir}")
    candidates = [
        d for d in local_dir.iterdir()
        if d.is_dir() and d.name.startswith(f"{version}+")
    ]
    if not candidates:
        fail(
            f"release/Local 下未找到版本 {version} 的构建产物目录 "
            f"（期望形如 {version}+<分支>.<提交数>.<短哈希>）。\n"
            f"请先手动执行本地构建: uv run python scripts/release_local.py {version}"
        )
    # 提交数多者视为较新构建；并列时取目录修改时间新者
    chosen = max(
        candidates,
        key=lambda d: (parse_build_count(d.name), d.stat().st_mtime),
    )
    return chosen


def validate_dir(serve_dir: Path) -> None:
    """校验 serve 目录包含联调必需文件；缺失报错退出（不做自动生成）。"""
    missing = [name for name in REQUIRED_FILES if not (serve_dir / name).is_file()]
    if missing:
        fail(
            f"目录 {serve_dir} 缺少必要文件: {', '.join(missing)}。\n"
            f"请先用本地构建生成: uv run python scripts/release_local.py "
            f"{serve_dir.name.split('+', 1)[0]}"
        )
    assets = [
        p.name for p in serve_dir.iterdir()
        if p.is_file() and p.suffix.lower() in (".exe", ".zip")
    ]
    if not assets:
        fail(f"目录 {serve_dir} 下未找到任何安装包/便携版产物（*.exe / *.zip）")
    # 校验清单可解析且与目录版本一致（防 serve 陈旧文件）
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
            f"  /releases/latest/download/versions.json\n"
            f"  /releases/download/{v_prefix}/latest-dev.json\n"
            f"  /releases/download/{v_prefix}/<asset>\n"
            f"assets: {', '.join(assets)}"
        )

    @app.get("/releases/latest/download/versions.json")
    def versions_index() -> FileResponse:
        return _serve_file("versions.json")

    @app.get("/releases/download/{req_version}/latest-dev.json")
    def latest_manifest(req_version: str) -> FileResponse:
        if not _same_version(req_version):
            raise HTTPException(status_code=404, detail="该版本未在本服务发布")
        return _serve_file("latest-dev.json")

    @app.get("/releases/download/{req_version}/{filename:path}")
    def download_asset(req_version: str, filename: str) -> FileResponse:
        if not _same_version(req_version):
            raise HTTPException(status_code=404, detail="该版本未在本服务发布")
        return _serve_file(filename)

    return app


def main() -> None:
    parser = argparse.ArgumentParser(
        description="本地更新链路联调 HTTP 服务（模拟远端发布源，GitHub 同构 URL）"
    )
    parser.add_argument("--version", required=True, help="要 serve 的版本号（如 99.0.0）")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT, help=f"监听端口（默认 {DEFAULT_PORT}）")
    parser.add_argument("--host", default="127.0.0.1", help="监听地址（默认 127.0.0.1）")
    parser.add_argument("--dir", type=Path, default=DEFAULT_DIR,
                        help=f"产物根目录（默认 {DEFAULT_DIR}）")
    args = parser.parse_args()

    version = args.version.lstrip("vV")
    serve_dir = pick_version_dir(args.dir, version)
    validate_dir(serve_dir)
    log("INFO", f"serve 目录: {serve_dir}")
    log("INFO", f"serve 版本: {version}")
    log("INFO", "已就绪（应用侧启动示例）:")
    log("INFO", f"  ROLLCALLER_UPDATE_BASE=http://{args.host}:{args.port} cargo tauri dev")
    log("INFO", "自检: "
                f"curl http://{args.host}:{args.port}/releases/latest/download/versions.json")

    app = build_app(serve_dir, version)
    import uvicorn
    uvicorn.run(app, host=args.host, port=args.port, log_level="info")


if __name__ == "__main__":
    main()
