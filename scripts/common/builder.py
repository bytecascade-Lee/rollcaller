#!/usr/bin/env python3
"""
构建模块：执行 cargo tauri build。

负责 target 感知的 release 目录定位、旧 bundle 清理、
CI 环境变量兼容（CI=1 会导致 tauri-cli clap 解析失败）。
"""

import os
import shutil
import subprocess
from pathlib import Path
from typing import Dict, List, Optional


class BuildError(Exception):
    pass


def release_dir(backend: Path, target: Optional[str]) -> Path:
    """根据是否指定 --target 返回对应的 release 目录。"""
    if target:
        return backend / "target" / target / "release"
    return backend / "target" / "release"


def build(
    root: Path,
    backend: Path,
    target: Optional[str],
    cli_cmd: List[str],
    cli_label: str,
    env_overrides: Optional[Dict[str, str]] = None,
) -> Path:
    """
    构建 Tauri 应用，返回 release 目录路径。

    env_overrides 会合并进子进程环境（如 VERSION / BRANCH_NAME，被 build.rs 读取后
    嵌入二进制）。
    """
    frontend = root / "frontend"
    if not (frontend / "node_modules").exists():
        raise BuildError(
            "frontend/node_modules 不存在，请先执行: cd frontend && pnpm install"
        )

    # tauri-cli 会把 CI 环境变量当作 --ci 参数的默认值，值为非 true/false（如 CI=1）时 clap 解析失败
    env = os.environ.copy()
    if env.get("CI", "").lower() not in ("", "true", "false"):
        del env["CI"]
    if env_overrides:
        env.update(env_overrides)

    release_dir_ = release_dir(backend, target)
    # 清掉旧 bundle，保证 nsis 目录下只有一个安装包
    shutil.rmtree(release_dir_ / "bundle", ignore_errors=True)

    cmd = [*cli_cmd, "build"]
    if target:
        cmd += ["--target", target]

    print(f">> 使用 tauri-cli: {cli_label}")
    print(f">> 执行: {' '.join(cmd)} (cwd={backend})")
    proc = subprocess.run(cmd, cwd=backend, env=env)
    if proc.returncode != 0:
        raise BuildError(f"cargo tauri build 失败，退出码 {proc.returncode}")
    return release_dir_
