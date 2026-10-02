#!/usr/bin/env python3
# -*- coding: UTF-8 -*-
"""
tauri-cli ：构造并执行 cargo tauri 的常见子命令（build / signer sign）。


CLI 来源：
    1) 仓库内置 paths.TAURI_CLI_PATH（<resources>/tauri/cargo-tauri.exe）
       调用形式 `<exe> tauri <子命令>`，与 cargo 派发外部子命令时的 argv 一致
    2) 找不到则回落系统 `cargo tauri <子命令>`（PATH / ~/.cargo/bin 上的 cargo-tauri）

工作目录恒为 BACKEND_DIR：
    - build：tauri.conf.json5 与 Cargo.toml 都在这里，是命令自身的要求
    - sign ：与 cwd 无关，用 BACKEND_DIR 避免依赖「cargo 派发子命令不需要 manifest」

环境变量：
    默认 env=None => 子进程继承父进程环境。
    `sign` 依靠此得到 TAURI_SIGNING_PRIVATE_KEY(_PASSWORD)，因此**不得**向 sign 传递 env。
    build 需要显式 env 并注入 VERSION / BRANCH_NAME，而删变量是继承做不到的。
    就地改 os.environ 也能达成同样效果，但会污染本进程、泄漏给之后所有子进程；
    显式传参只作用于这一次调用，调用点也自解释。
    ⚠️ subprocess 的 env= 是**替换**而非合并：调用方需自行 os.environ.copy() 后再改。
"""

import subprocess
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple

from common.constant.paths import BACKEND_DIR, TAURI_CLI_PATH
from common.logger import info, warning, debug


class TauriError(Exception):
    """tauri-cli 调用失败：二进制不可用，或命令返回非 0"""


def _cli_source() -> Tuple[str, List[str]]:
    """返回 (显示名, 命令前缀)，并把用的是哪一个打出来"""
    if TAURI_CLI_PATH.is_file():
        info(f"tauri-cli: 仓库内置 {TAURI_CLI_PATH}")
        return f"仓库内置 {TAURI_CLI_PATH}", [str(TAURI_CLI_PATH), "tauri"]
    warning(
        f"未找到仓库内置的 tauri-cli（{TAURI_CLI_PATH}），回落到系统 cargo tauri；"
        f"两者可能不是同一版本，发布构建请先备好内置 CLI"
    )
    return "系统 cargo tauri", ["cargo", "tauri"]


def _run(args: Sequence[str], env: Optional[Dict[str, str]] = None) -> None:
    """在 BACKEND_DIR 下执行一条 tauri 命令；启动失败或退出码非 0 抛 TauriError"""
    _label, prefix = _cli_source()  # 显示名已在 _cli_source 内打印
    cmd = [*prefix, *args]
    debug(f"执行: {' '.join(cmd)} (cwd={BACKEND_DIR})")
    try:
        proc = subprocess.run(cmd, cwd=BACKEND_DIR, env=env)
    except FileNotFoundError as e:
        raise TauriError(f"tauri-cli 不可用: {e}")
    if proc.returncode != 0:
        raise TauriError(f"tauri {' '.join(args)} 失败（退出码 {proc.returncode}）")


def build(
    target: str,
    *,
    debug_mode: bool = False,
    bundle: bool = True,
    env: Optional[Dict[str, str]] = None,
) -> None:
    """执行 `tauri build`。

    target：**必填**的完整 Rust 三元组，这样产物恒落在 backend/target/<triple>/release/，
        调用方取产物时无需再按「是否指定 target」分支。
    debug_mode：True ⇒ 追加 --debug，产物落到 .../<triple>/debug/。
    bundle：默认 True（正常打包）；False ⇒ 追加 --no-bundle（仅 debug 构建用）。
    env：见模块 doc——默认 None 继承父进程；显式给出则是**替换**语义。
    """
    args = ["build", "--target", target]
    if debug_mode:
        args.append("--debug")
    if not bundle:
        args.append("--no-bundle")
    _run(args, env=env)


def sign(path: Path) -> None:
    """执行 `tauri signer sign <path>`（minisign 签名，产出 <path>.sig）。

    密钥经环境变量读取（TAURI_SIGNING_PRIVATE_KEY / _PASSWORD）⇒ 此处不传 env，子进程
    继承父进程即可看见；是否齐备由调用方自行校验（见 common/signer.py）。
    这里只跑命令与错误归一——`.sig` 是否存在、是否非空，由调用方断言。
    路径校验从宽：绝对路径直接放行；相对路径若能按「当前工作目录」定位到文件，先规范化成
    绝对路径再传下去（cwd 恒为 BACKEND_DIR，相对路径原样传会被解析到那里）；两者都不成立
    才报错。绝对路径但文件不存在时同样放行，交给 tauri 自己报错。
    """
    if not path.is_absolute():
        if not path.is_file():
            raise TauriError(
                f"sign 的路径既不是绝对路径、也定位不到文件: {path}"
                f"（本模块 cwd 固定为 {BACKEND_DIR}，相对路径会被解析到那里，请传绝对路径）"
            )
        path = path.resolve()
        debug(f"相对路径已按当前工作目录规范化为: {path}")
    _run(["signer", "sign", str(path)])


def resolve(root: Path) -> Tuple[str, List[str]]:
    """过渡，旧 CLI 来源查询，返回 (显示名, 命令前缀)。

    在调用方迁移完成前保留，供 build_local.py / build_ci.py / common/signer.py 继续使用
    （root 形参已无用，仅为不破坏旧调用）。与旧实现的差别：命令前缀改为**绝对路径**——
    旧版返回相对路径，只有 cwd 恰好在仓库根时才碰巧可用。
    等这些调用方都改走 build() / sign() 之后，本函数连同 root 形参一并删除。
    """
    return _cli_source()
