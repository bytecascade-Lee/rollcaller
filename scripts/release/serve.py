#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
本地更新源 HTTP 服务

服务固定监听 http://localhost:14652（与后端 DEVELOP 常量一致），只 serve
一个构建目录。URL 与后端 `common/constant/update.rs` 的 DEVELOP 常量同构：

    GET /rollcaller/releases/latest/download/versions.json
    GET /rollcaller/releases/latest/download/latest-develop.json
    GET /rollcaller/releases/download/v<version>/latest-develop.json
    GET /rollcaller/releases/download/v<version>/<asset>

[TODO] Range 206
限速路径当前不支持 `Range` 请求（响应头 `Accept-Ranges: none`）。后期若在
GitHub 源上实现断点续传 / 多线程分段下载，客户端会携带 `Range` 请求头，
届时需为本服务补上单区间（多线程则需多区间）206 支持，并让限速同样作用于
每个分段响应；否则本地链路会「看起来能续传」地与真实源行为不一致。
"""

import time
from collections.abc import Iterator
from pathlib import Path

from common.enums.source import Source
from common.logger import info
from release.urls import manifest_name
from release.versions_index import FILENAME

HOST = "localhost"
PORT = 14652
PREFIX = "/rollcaller"
# 默认限速(KB/s，1024 进制)
DEFAULT_RATE_KB = 4096
# 限速分块上限
CHUNK_MAX = 64 * 1024
# 不限速时的产出块
CHUNK_UNLIMITED = 256 * 1024

# serve 目录内必须存在的文件
REQUIRED_FILES = (manifest_name(Source.DEVELOP), FILENAME)


class ServeError(Exception):
    """本地服务目录不满足要求"""


def validate_dir(serve_dir: Path) -> None:
    """校验 serve 目录包含联调必需文件；缺失报错"""
    if not serve_dir.is_dir():
        raise ServeError(f"serve 目录不存在: {str(serve_dir)!r}")
    missing = [name for name in REQUIRED_FILES if not (serve_dir / name).is_file()]
    if missing:
        raise ServeError(
            f"目录 {str(serve_dir)!r} 缺少必要文件: {', '.join(missing)}；"
            f"请先完成本地发布写入清单与版本索引"
        )


def throttle_plan(rate_kb: int) -> tuple[int, float | None]:
    """由限速值推出分块大小与每块等待时间；`rate_kb <= 0` 即不限速"""
    if rate_kb <= 0:
        return CHUNK_UNLIMITED, None
    rate = rate_kb * 1024
    chunk = max(1, min(CHUNK_MAX, rate // 2))
    return chunk, chunk / rate


def rate_desc(rate_kb: int) -> str:
    """限速描述，展示实际分块与 tick"""
    if rate_kb <= 0:
        return "unlimited (--rate 0)"
    chunk, delay = throttle_plan(rate_kb)
    return f"{rate_kb} KB/s (1024-based; chunk {chunk} B, tick {delay * 1000:.1f} ms)"


def throttled_stream(target: Path, rate_kb: int) -> Iterator[bytes]:
    """按 `rate_kb` KB/s 分块产出文件内容"""
    chunk, delay = throttle_plan(rate_kb)
    rate = rate_kb * 1024 if (delay is not None) else None
    start = time.monotonic()
    sent = 0
    with target.open("rb") as f:
        while block := f.read(chunk):
            yield block
            sent += len(block)
            if rate is None:
                continue
            lag = start + sent / rate - time.monotonic()
            if lag > 0:
                time.sleep(lag)


def build_app(serve_dir: Path, version: str, rate_kb: int = DEFAULT_RATE_KB):
    """构造 FastAPI 应用（延迟 import，避免无 fastapi 时脚本 -h 也报错）"""
    from fastapi import FastAPI, HTTPException
    from fastapi.responses import FileResponse, PlainTextResponse, Response, StreamingResponse

    app = FastAPI(title="rollcaller local update server", docs_url=None, redoc_url=None)
    v_prefix = f"v{version}"
    latest = f"{PREFIX}/releases/latest/download"
    download = f"{PREFIX}/releases/download"

    def _same_version(req_version: str) -> bool:
        """请求路径中的版本号（可能带前导 v）须与 serve 版本一致"""
        return req_version.lstrip("vV") == version

    def _resolve(name: str) -> Path:
        """把请求名解析为 serve 目录内的真实文件路径（防路径穿越；仅文件，不递归）"""
        target = (serve_dir / name).resolve()
        if not target.is_relative_to(serve_dir.resolve()) or not target.is_file():
            raise HTTPException(status_code=404, detail="文件不存在")
        return target

    def _serve_file(name: str) -> FileResponse:
        """清单/索引等小文件：全速 FileResponse（限速只作用于产物直链）"""
        return FileResponse(_resolve(name))

    def _serve_asset(name: str) -> Response:
        """产物直链：限速走分块 StreamingResponse，不限速则退化为全速 FileResponse"""
        target = _resolve(name)
        if rate_kb <= 0:
            return FileResponse(target)
        return StreamingResponse(
            throttled_stream(target, rate_kb),
            media_type="application/octet-stream",
            headers={
                "Content-Length": str(target.stat().st_size),
                "Content-Disposition": f'attachment; filename="{target.name}"',
                # 限速路径不支持 Range/206（见模块 docstring 的 TODO）
                "Accept-Ranges": "none",
            },
        )

    @app.get("/")
    def index() -> PlainTextResponse:
        assets = sorted(p.name for p in serve_dir.iterdir() if p.is_file())
        return PlainTextResponse(
            "rollcaller local update server\n"
            f"serve version: {version} ({serve_dir})\n"
            f"throttle: {rate_desc(rate_kb)}\n"
            "routes:\n"
            f"    {latest}/versions.json\n"
            f"    {latest}/latest-develop.json\n"
            f"    {download}/{v_prefix}/latest-develop.json\n"
            f"    {download}/{v_prefix}/<asset>\n"
            f"assets:\n    {',\n    '.join(assets)}"
        )

    @app.get(f"{latest}/versions.json")
    def versions_index_route() -> FileResponse:
        """版本索引（每次检查都拉取，不缓存）"""
        return _serve_file("versions.json")

    @app.get(f"{latest}/latest-develop.json")
    def latest_manifest_route() -> FileResponse:
        """latest/download 形态（后端当前未消费，serve 同一文件兼容）"""
        return _serve_file("latest-develop.json")

    @app.get(f"{download}/{{req_version}}/latest-develop.json")
    def specified_manifest_route(req_version: str) -> FileResponse:
        """指定版本清单（check.rs 实际消费的目标版本清单）"""
        if not _same_version(req_version):
            raise HTTPException(status_code=404, detail="该版本未在本服务发布")
        return _serve_file("latest-develop.json")

    @app.get(f"{download}/{{req_version}}/{{filename:path}}")
    def download_asset(req_version: str, filename: str) -> Response:
        """清单内产物直链（按 rate_kb 限速）"""
        if not _same_version(req_version):
            raise HTTPException(status_code=404, detail="该版本未在本服务发布")
        return _serve_asset(filename)

    return app


def serve(directory: Path, version: str, rate_kb: int = DEFAULT_RATE_KB) -> None:
    """前台驻留本地更新源；Ctrl+C 由入口的 graceful_sigint 收场"""
    validate_dir(directory)
    info(f"serve 目录: {directory}")
    info(f"serve 版本: {version}")
    info(f"监听: http://{HOST}:{PORT}{PREFIX}/（URL 与后端 DEVELOP 常量同构）")
    info(f"限速: {rate_desc(rate_kb)}（仅产物直链；清单/索引全速）")

    app = build_app(directory, version, rate_kb)
    import uvicorn

    uvicorn.run(app, host=HOST, port=PORT, log_level="info")
