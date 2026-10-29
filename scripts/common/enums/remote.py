#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
远端角色
"""

from enum import StrEnum


class RemoteRole(StrEnum):
    """一个远端 URL 与本仓库的关系"""

    # 规范仓库本身
    CANONICAL = "canonical"
    # 只读镜像 CNB
    MIRROR = "mirror"
    # github.com 上 owner 不同的同名仓库，即 fork
    FORK = "fork"
    # 与本仓库无关
    UNRELATED = "unrelated"
