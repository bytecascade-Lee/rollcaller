#!/usr/bin/env python3
"""
本地更新链路联调 HTTP 服务（模拟远端发布源）。

用法:
    uv run python scripts/local_update_source_server.py <版本号> [--dir <dir>] [--rate <KB/s>]

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

限速（`--rate`，KB/s，1024 进制）：仅作用于**产物直链**（setup.exe / portable.zip），
让联调时能观察到进度推进、并来得及点"取消下载"；清单/索引等小文件仍全速返回，
便于区分"慢在产物传输"还是"慢在检查阶段"。`--rate 0` 即不限速。实现为
`StreamingResponse` + 同步生成器 for+sleep（见 `throttled_stream`），替代全速的
`FileResponse`，因此响应头需自行补齐（尤其是 Content-Length，见 `_serve_asset`）。

[TODO] Range、206
限速路径当前**明确不支持** `Range` 请求（响应头 `Accept-Ranges: none`），
与 CNB 源行为一致，与后端现状匹配（后端暂不实现断点续传，见 service/update/download.rs）。
后期若在 GitHub 源上实现断点续传 / 多线程分段下载，客户端会携带 `Range` 请求头，
届时需为本服务补上单区间（多线程则需多区间）206 支持，并让限速同样作用于每个分段响应；
否则本地链路会"看起来能续传"地与真实源行为不一致，无法覆盖该路径的联调。

配套（应用侧）：以后端 UpdateSource::Develop 访问本服务即走本地更新链路。
"""

import argparse
import json
import time
from collections.abc import Iterator
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

# 产物直链默认限速（KB/s，1024 进制）：本地安装包约 12~15MB，4MB/s 约 3~4s，
# 进度条看得见且不至于干等；想更快用 --rate 8192，想看清进度用 --rate 512。
DEFAULT_RATE_KB = 4096
# 限速分块上限（字节）：每 tick 时长 = chunk / rate，由 throttle_plan 自适应分块使该值 <= 0.5s，
# 以免低速率下（如 --rate 10）拉出秒级间隔、逼近后端下载客户端的空闲读超时
# （backend src/state/http_client.rs 的 read_timeout = 60s）。
CHUNK_MAX = 64 * 1024
# 不限速时的产出块（走 StreamingResponse 的全速路径）
CHUNK_UNLIMITED = 256 * 1024

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


def throttle_plan(rate_kb: int) -> tuple[int, float | None]:
    """由限速值推出 (分块字节数, 每块配速秒数)；`rate_kb <= 0` 即不限速（秒数为 None）。

    分块随速率自适应，使每 tick 时长 <= 0.5s：若固定 64 KiB 分块，`--rate 10`（10 KiB/s）
    会变成每块睡 6.4s——虽然仍在 60s 空闲读超时之内，但粒度粗且容易误判为"卡死"。
    """
    if rate_kb <= 0:
        return CHUNK_UNLIMITED, None
    rate = rate_kb * 1024
    chunk = max(1, min(CHUNK_MAX, rate // 2))
    return chunk, chunk / rate


def rate_desc(rate_kb: int) -> str:
    """限速描述（启动日志与索引页共用），顺带暴露实际分块与 tick 以便确认配置生效。"""
    if rate_kb <= 0:
        return "unlimited (--rate 0)"
    chunk, delay = throttle_plan(rate_kb)
    return f"{rate_kb} KB/s (1024-based; chunk {chunk} B, tick {delay * 1000:.1f} ms)"


def throttled_stream(target: Path, rate_kb: int) -> Iterator[bytes]:
    """按 `rate_kb` KB/s 分块产出文件内容（**同步**生成器，函数体内不含任何 await）。

    # 为何 for+sleep 在这里安全
    同步生成器会被 starlette 用 `iterate_in_threadpool` 逐块在 worker 线程拉取
    （starlette/responses.py 的 `StreamingResponse.__init__`：非 AsyncIterable 的 content
    一律包成线程池迭代器），因此这里的 `time.sleep` **不会阻塞事件循环**——清单/索引等
    小请求仍即时响应。反过来说，本生成器（及使用它的路由）**必须保持同步**：写成
    `async def` + `time.sleep` 会阻塞整个事件循环，把清单请求一起拖死。

    # 配速
    按"起始时刻 + 已发字节 / 速率"推算当前应处的时刻，再 sleep 到该时刻；而非每块固定
    `sleep(delay)`。后者会把线程调度与客户端消费的耗时逐块累加，实际速率持续低于标称值。
    客户端消费慢导致 lag 为负时不再额外等待（此时瓶颈在客户端，不该由服务端再加惩罚）。
    """
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
    """构造 FastAPI 应用（延迟 import，避免无 fastapi 时脚本 -h 也报错）。"""
    from fastapi import FastAPI, HTTPException
    from fastapi.responses import FileResponse, PlainTextResponse, Response, StreamingResponse

    app = FastAPI(title="rollcaller local update server", docs_url=None, redoc_url=None)
    v_prefix = f"v{version}"
    latest = f"{PREFIX}/releases/latest/download"
    download = f"{PREFIX}/releases/download"

    def _same_version(req_version: str) -> bool:
        """请求路径中的版本号（可能带前导 v）须与 serve 版本一致。"""
        return req_version.lstrip("vV") == version

    def _resolve(name: str) -> Path:
        """把请求名解析为 serve 目录内的真实文件路径（防路径穿越；仅文件，不递归）。"""
        target = (serve_dir / name).resolve()
        if not target.is_relative_to(serve_dir.resolve()) or not target.is_file():
            raise HTTPException(status_code=404, detail="文件不存在")
        return target

    def _serve_file(name: str) -> FileResponse:
        """清单/索引等小文件：全速 FileResponse（限速只作用于产物直链）。"""
        return FileResponse(_resolve(name))

    def _serve_asset(name: str) -> Response:
        """产物直链：限速走分块 StreamingResponse，不限速则退化为全速 FileResponse。

        换掉 FileResponse 的代价要自行补齐：
        - **Content-Length 必给**：StreamingResponse 不给长度时 HTTP/1.1 会退化成
          `Transfer-Encoding: chunked`，客户端 `response.content_length()` 拿到 None，
          前端进度条的总长（DownloadProgress.total）就丢了。
        - MIME 不推断：本服务只 serve exe/zip，直接给 application/octet-stream。
        """
        target = _resolve(name)
        if rate_kb <= 0:
            return FileResponse(target)
        return StreamingResponse(
            throttled_stream(target, rate_kb),
            media_type="application/octet-stream",
            headers={
                "Content-Length": str(target.stat().st_size),
                "Content-Disposition": f'attachment; filename="{target.name}"',
                # 限速路径不支持 Range/206（见模块 docstring 的 TODO）：
                # 显式声明 none，免得客户端误以为可续传。
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
    def download_asset(req_version: str, filename: str) -> Response:
        """清单内产物直链（setup.exe / portable.zip 下载，按 --rate 限速）。

        注：`@app.get`（FastAPI 的 APIRoute）只注册 GET，未像 starlette 的 Route 那样自动
        附带 HEAD，HEAD 一律 405（本服务历来如此，改动前后一致），故无需区分请求方法。
        若将来显式放开 HEAD，必须让它旁路 `_serve_asset`：限速生成器不感知请求方法，
        会替无 body 的 HEAD 把整个文件"睡"一遍才结束。
        """
        if not _same_version(req_version):
            raise HTTPException(status_code=404, detail="该版本未在本服务发布")
        return _serve_asset(filename)

    return app


def main() -> None:
    parser = argparse.ArgumentParser(
        description="本地更新链路联调 HTTP 服务（模拟远端发布源，URL 与后端 DEVELOP 常量同构）"
    )
    parser.add_argument("version", help="要 serve 的核心版本号（如 99.0.0）")
    parser.add_argument("--dir", type=Path, default=DEFAULT_DIR,
                        help=f"产物根目录（默认 {DEFAULT_DIR}）")
    parser.add_argument("--rate", type=int, default=DEFAULT_RATE_KB,
                        help=f"产物直链限速，KB/s（1024 进制）；0 = 不限速（默认 {DEFAULT_RATE_KB}）")
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
    log("INFO", f"限速: {rate_desc(args.rate)}（仅产物直链；清单/索引全速）")
    log("INFO", "自检: "
                f"curl http://{HOST}:{PORT}{PREFIX}/releases/latest/download/versions.json")

    app = build_app(serve_dir, version, args.rate)
    import uvicorn
    uvicorn.run(app, host=HOST, port=PORT, log_level="info")


if __name__ == "__main__":
    main()
