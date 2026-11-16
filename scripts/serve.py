#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
本地更新源 HTTP 服务

监听 http://localhost:14652/rollcaller（与后端 `updater/urls.rs` 的 DEVELOP 常量逐字符一致），
只 serve 一个构建目录；路由与响应头见 `serve.app`，首页见 `serve.page`。

用法：
    just serve                                   # 最近修改且完整的构建目录，版本从清单读
    just serve 0.8.5+develop.2323.3bec9c7d82     # 指定 release/Local 下的目录名
    just serve --rate 0                          # 不限速

服务的输入是「目录 + 里面的清单」，与发布动作无关：旧版本目录照样能 serve，
重新起停不会重写清单（时间戳与签名都保持原样）。
"""

import argparse
import sys
from pathlib import Path

from common.constant.paths import LOCAL_RELEASE_DIR
from common.constant.updater import DEVELOP_BASE, DEVELOP_HOST, DEVELOP_PORT
from common.logger import error, info
from serve.app import build_app
from serve.source import ServeError, inspect, validate_dir
from serve.throttle import DEFAULT_RATE_KB, Throttle
from util.sigint import graceful_sigint
from util.str_format import join_repr


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="本地更新源：把一个构建目录 serve 成 Develop 更新源")
    _ = parser.add_argument(
        "directory",
        nargs="?",
        metavar="DIR",
        help="构建目录，或 release/Local 下的目录名；缺省取最近修改且完整的一个",
    )
    _ = parser.add_argument(
        "-r", "--rate", type=int, default=DEFAULT_RATE_KB, metavar="KBPS", help="产物直链限速 KB/s，0 表示不限速"
    )
    return parser.parse_args()


def latest_complete() -> Path:
    """release/Local 下最近修改、且清单与索引都在的构建目录"""
    if not LOCAL_RELEASE_DIR.is_dir():
        raise ServeError(f"{str(LOCAL_RELEASE_DIR)!r} 不存在，先跑 just build 与 just release")
    builds = sorted(
        (path for path in LOCAL_RELEASE_DIR.iterdir() if path.is_dir()),
        key=lambda path: path.stat().st_mtime,
        reverse=True,
    )
    if not builds:
        raise ServeError(f"{str(LOCAL_RELEASE_DIR)!r} 下没有任何构建目录，先跑 just build 与 just release")

    for build in builds:
        try:
            validate_dir(build)
        except ServeError:
            continue
        return build
    names = join_repr([build.name for build in builds])
    raise ServeError(f"{str(LOCAL_RELEASE_DIR)!r} 下的构建都不完整（缺清单或索引），先跑 just release：{names}")


def resolve_dir(argument: str | None) -> Path:
    """命令行给的目录；缺省取最近修改且完整的构建目录"""
    if argument is None:
        return latest_complete().resolve()

    candidate = Path(argument)
    if candidate.is_dir():
        return candidate.resolve()

    under_local = LOCAL_RELEASE_DIR / argument
    if under_local.is_dir():
        return under_local.resolve()
    raise ServeError(f"{argument!r} 既不是目录，也不是 {str(LOCAL_RELEASE_DIR)!r} 下的名字")


def main() -> int:
    args = parse_args()
    serve_dir = resolve_dir(args.directory)
    validate_dir(serve_dir)

    throttle = Throttle.of(args.rate)
    view = inspect(serve_dir, throttle.describe())

    info(f"serve 目录: {serve_dir}")
    info(f"serve 版本: {view.version}")
    info(f"监听: {DEVELOP_BASE}/（URL 与后端 DEVELOP 常量同构）")
    info(f"首页: http://{DEVELOP_HOST}:{DEVELOP_PORT}/")
    info(f"限速: {throttle.describe()}（仅产物直链；清单/索引全速）")

    app = build_app(serve_dir, view.version, throttle)
    import uvicorn

    uvicorn.run(app, host=DEVELOP_HOST, port=DEVELOP_PORT, log_level="info")
    return 0


if __name__ == "__main__":
    with graceful_sigint() as is_interrupted:
        try:
            sys.exit(main())
        except ServeError as err:
            if not is_interrupted():
                error(f"本地更新源启动失败: {err}")
                sys.exit(1)
