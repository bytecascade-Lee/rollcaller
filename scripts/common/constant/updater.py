#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
自动更新源的契约常量
"""

# DEVELOP 源：本地联调服务的监听地址
DEVELOP_HOST = "localhost"
DEVELOP_PORT = 14652
DEVELOP_PREFIX = "/rollcaller"
DEVELOP_BASE = f"http://{DEVELOP_HOST}:{DEVELOP_PORT}{DEVELOP_PREFIX}"

# 版本索引文件名
VERSIONS_INDEX_FILENAME = "versions.json"
