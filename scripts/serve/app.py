#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
HTTP 装配

6 条对外路由与后端 `updater/urls.rs` 的模板同构：

    GET /rollcaller/releases/latest/download/versions.json
    GET /rollcaller/releases/latest/download/latest-develop.json
    GET /rollcaller/releases/download/v<version>/latest-develop.json
    GET /rollcaller/releases/download/v<version>/<asset>
    GET /   与  GET /rollcaller   首页

清单与索引全速，产物直链按 `Throttle` 限速。

[TODO] Range 206
限速路径当前不支持 `Range` 请求（响应头 `Accept-Ranges: none`）。后期若在
GitHub 源上实现断点续传 / 多线程分段下载，客户端会携带 `Range` 请求头，
届时需为本服务补上单区间（多线程则需多区间）206 支持，并让限速同样作用于
每个分段响应；否则本地链路会「看起来能续传」地与真实源行为不一致。
"""

from pathlib import Path
from typing import TYPE_CHECKING

from common.constant.updater import DEVELOP_PREFIX, VERSIONS_INDEX_FILENAME
from serve import page
from serve.source import MANIFEST_FILENAME, ServeError, inspect
from serve.throttle import Throttle

if TYPE_CHECKING:
    from fastapi import FastAPI


def build_app(serve_dir: Path, version: str, throttle: Throttle) -> "FastAPI":
    """构造 FastAPI 应用（延迟 import，避免无 fastapi 时脚本 -h 也报错）"""
    from fastapi import FastAPI, HTTPException
    from fastapi.responses import FileResponse, HTMLResponse, PlainTextResponse, Response, StreamingResponse

    root = serve_dir.resolve()
    download = f"{DEVELOP_PREFIX}/releases/download"
    latest = f"{DEVELOP_PREFIX}/releases/latest/download"

    app = FastAPI(title="rollcaller local update server", docs_url=None, redoc_url=None)

    def _same_version(req_version: str) -> bool:
        """请求路径中的版本号（可带前导 v）须与 serve 版本一致"""
        return req_version.removeprefix("v") == version

    def _resolve(name: str) -> Path:
        """把请求名解析为 serve 目录内的真实文件路径（防路径穿越；仅文件，不递归）"""
        target = (root / name).resolve()
        if not target.is_relative_to(root) or not target.is_file():
            raise HTTPException(status_code=404, detail="文件不存在")
        return target

    def _serve_file(name: str) -> FileResponse:
        """清单/索引等小文件：全速 FileResponse（限速只作用于产物直链）"""
        return FileResponse(_resolve(name))

    def _serve_asset(name: str) -> Response:
        """产物直链：限速走分块 StreamingResponse，不限速则退化为全速 FileResponse"""
        target = _resolve(name)
        if not throttle.limited:
            return FileResponse(target)
        return StreamingResponse(
            throttle.stream(target),
            media_type="application/octet-stream",
            headers={
                "Content-Length": str(target.stat().st_size),
                "Content-Disposition": f'attachment; filename="{target.name}"',
                # 限速路径不支持 Range/206（见模块 docstring 的 TODO）
                "Accept-Ranges": "none",
            },
        )

    @app.get("/")
    @app.get(DEVELOP_PREFIX)
    def index() -> Response:
        """首页：/ 与 /rollcaller 同一页，每次请求重新采集磁盘状态"""
        try:
            view = inspect(root, throttle.describe())
        except ServeError as error:
            return PlainTextResponse(f"本地更新源无法出页：{error}", status_code=503)
        return HTMLResponse(page.render(view), headers={"Cache-Control": "no-store"})

    @app.get(f"{latest}/{VERSIONS_INDEX_FILENAME}")
    def versions_index_route() -> FileResponse:
        """版本索引（每次检查都拉取，不缓存）"""
        return _serve_file(VERSIONS_INDEX_FILENAME)

    @app.get(f"{latest}/{MANIFEST_FILENAME}")
    def latest_manifest_route() -> FileResponse:
        """latest/download 形态（后端当前未消费，serve 同一文件兼容）"""
        return _serve_file(MANIFEST_FILENAME)

    @app.get(f"{download}/{{req_version}}/{MANIFEST_FILENAME}")
    def specified_manifest_route(req_version: str) -> FileResponse:
        """指定版本清单（check.rs 实际消费的目标版本清单）"""
        if not _same_version(req_version):
            raise HTTPException(status_code=404, detail="该版本未在本服务发布")
        return _serve_file(MANIFEST_FILENAME)

    @app.get(f"{download}/{{req_version}}/{{filename:path}}")
    def download_asset(req_version: str, filename: str) -> Response:
        """清单内产物直链（按 rate 限速）"""
        if not _same_version(req_version):
            raise HTTPException(status_code=404, detail="该版本未在本服务发布")
        return _serve_asset(filename)

    return app
