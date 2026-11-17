#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
发布端 HTTP 读取
"""

import urllib.error
import urllib.request

DEFAULT_TIMEOUT = 30


def request(url: str, *, method: str = "GET", timeout: int = DEFAULT_TIMEOUT) -> tuple[int, bytes]:
    """发一个请求，返回 (状态码, 响应体)；网络错误的状态码为 0，响应体是错误说明"""
    req = urllib.request.Request(url, method=method, headers={"User-Agent": "rollcaller-release-tool"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as response:
            return response.status, response.read()
    except urllib.error.HTTPError as e:
        return e.code, b""
    except (urllib.error.URLError, TimeoutError, OSError) as e:
        return 0, str(e).encode("utf-8", "replace")
