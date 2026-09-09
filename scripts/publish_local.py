#!/usr/bin/env python3
"""
本地发布脚本：为 release/Local 下某版本的构建产物目录生成联调清单。

用法:
    uv run python scripts/publish_local.py <版本号> [--output-dir <dir>]
        [--severity normal|important|critical] [--serve-base <url>]

在产物目录（release/Local/<版本号>+<构建信息>/，同版本多目录取提交数最多者）内
生成两个文件（不联网、不碰 git）：
    - latest-develop.json   # v2 结构（与 latest-v2.json / Rust UpdateManifest 一致）；
                            #   portable/develop 载荷的 signature 取自打包时生成的 .sig
                            #   （develop = Develop 直更载荷，见 build_local 的 develop 产物）
    - versions.json         # 裸数组：扫描 release/Local 全部已构建版本，
                            #   severity 沿仓库源 resources/update/versions.json 标定，
                            #   当前版本取 --severity，按语义化版本倒序

产物 URL 指向 <serve-base>/releases/download/v<版本>/<文件名>（与后端
constant/update.rs 的 DEVELOP 系列同构，默认 serve-base 与其一致）。

联调：`uv run python scripts/serve_local.py <版本号>` 即可本地 serve 本目录。
"""

import argparse
import json
from pathlib import Path

from common import manifest, versions_index, version as version_mod
from common.logger import log

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_DIR = versions_index.default_local_dir(ROOT)
# 仓库维护的版本索引（severity 唯一标定处；本地扫描对同名版本沿用其 severity）
REPO_VERSIONS_INDEX = ROOT / "resources" / "update" / "versions.json"
# 本地清单文件名（serve_local 校验该文件必须存在；后端 DEVELOP 常量同名）
MANIFEST_NAME = "latest-develop.json"
# 与 backend constant/update.rs DEVELOP 系列一致
DEFAULT_SERVE_BASE = "http://localhost:14652/rollcaller"


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def display(path: Path) -> str:
    """日志用路径：仓库内显示相对路径，仓库外（如自定义 --output-dir）显示绝对路径。"""
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def repo_severity_map() -> dict:
    """读取仓库源 versions.json → {版本: severity}；文件缺失/损坏返回 {}。"""
    try:
        return versions_index.read_entries(REPO_VERSIONS_INDEX)
    except versions_index.VersionIndexError:
        return {}


def build_payloads(out_dir: Path, release_version: str, serve_base: str) -> dict:
    """按产物目录实况构造 {arch: {nsis, portable}} 载荷（sha256/size 实算）。"""
    payloads = {}
    for arch in ("x86_64", "arm64"):
        setups = list(out_dir.glob(f"rollcaller-*-windows-{arch}-setup.exe"))
        if not setups:
            continue
        setup_path = setups[0]
        sig_raw = manifest.read_sig_text(setup_path.with_name(setup_path.name + ".sig"))
        url = f"{serve_base}/releases/download/v{release_version}/{setup_path.name}"
        entry = {"nsis": manifest.build_artifact(url, setup_path, sig_raw)}

        portables = list(out_dir.glob(f"rollcaller-*-windows-{arch}-portable.zip"))
        if portables:
            zip_path = portables[0]
            zip_sig_raw = manifest.read_sig_text(zip_path.with_name(zip_path.name + ".sig"))
            purl = f"{serve_base}/releases/download/v{release_version}/{zip_path.name}"
            entry["portable"] = manifest.build_artifact(purl, zip_path, zip_sig_raw)

        develops = list(out_dir.glob(f"rollcaller-*-windows-{arch}-develop.zip"))
        if develops:
            dzip = develops[0]
            dzip_sig_raw = manifest.read_sig_text(dzip.with_name(dzip.name + ".sig"))
            durl = f"{serve_base}/releases/download/v{release_version}/{dzip.name}"
            entry["develop"] = manifest.build_artifact(durl, dzip, dzip_sig_raw)
        payloads[arch] = entry
    if not payloads:
        fail(f"产物目录 {out_dir} 下未找到任何 setup 安装包，无法生成清单")
    return payloads


def publish(
    version_arg: str,
    out_dir: Path = None,
    severity: str = "normal",
    serve_base: str = DEFAULT_SERVE_BASE,
    local_root: Path = None,
) -> Path:
    """为版本产物目录生成 latest-develop.json 与 versions.json。

    Args:
        version_arg: 版本号（可带 v）；取 core 版本号作为清单 version 与 URL 段
        out_dir: 产物目录；缺省按 local_root 下同版本目录自动定位
        severity: 当前版本的严重级别（normal/important/critical）
        serve_base: 清单产物 URL 的服务器 base（与 serve_local 一致）
        local_root: 本地产物根目录（默认 release/Local），out_dir 缺省时用于定位

    Returns:
        产物目录 Path
    """
    try:
        release_version = version_mod.validate(version_arg, min_level=None)
    except version_mod.VersionError as e:
        fail(str(e))
    if "+" in release_version:
        fail(f"清单版本应为 core 版本号（不带 +build）: {version_arg!r}")

    local_root = local_root or DEFAULT_DIR
    out_dir = out_dir or versions_index.pick_version_dir(local_root, release_version)
    log("INFO", f"清单目标目录: {out_dir}")

    payloads = build_payloads(out_dir, release_version, serve_base)

    notes = f"本地测试发布 {out_dir.name}"
    latest = manifest.build_latest_json(
        version=release_version,
        notes=notes,
        severity=severity,
        payloads=payloads,
    )
    latest_path = out_dir / MANIFEST_NAME
    latest_path.write_text(json.dumps(latest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    log("INFO", f"已生成 {display(latest_path)}（serve_base={serve_base}）")

    # versions.json：只有当前版本，等级取 --severity
    severity_map = {release_version: severity}
    entries = versions_index.build_entries(severity_map)
    versions_path = versions_index.write_entries(out_dir / "versions.json", entries)
    log("INFO", f"已生成 {display(versions_path)}（{len(entries)} 个版本条目）")
    return out_dir


def main() -> None:
    parser = argparse.ArgumentParser(
        description="为本地构建产物生成 latest-develop.json 与 versions.json（不联网不碰 git）"
    )
    parser.add_argument(
        "version",
        help="版本号，可带 v；与产物目录的 core 版本一致",
    )
    parser.add_argument(
        "--output-dir",
        default=None,
        help="产物目录（release/Local/<版本>+<构建>/）；缺省按版本自动定位",
    )
    parser.add_argument(
        "--severity",
        choices=manifest.SEVERITY_LEVELS,
        default="normal",
        help="当前版本的严重级别（默认 normal；critical 表示强制更新）",
    )
    parser.add_argument(
        "--serve-base",
        default=DEFAULT_SERVE_BASE,
        help=f"清单中产物 URL 的服务器 base（默认 {DEFAULT_SERVE_BASE}）",
    )
    parser.add_argument(
        "--dir",
        default=str(DEFAULT_DIR),
        help=f"本地产物根目录（默认 {DEFAULT_DIR}），仅 --output-dir 缺省时用于定位",
    )
    args = parser.parse_args()

    out_dir = publish(
        args.version,
        out_dir=Path(args.output_dir) if args.output_dir else None,
        severity=args.severity,
        serve_base=args.serve_base,
        local_root=Path(args.dir),
    )
    log("INFO", f"本地发布完成。联调: uv run python scripts/serve_local.py {args.version}")


if __name__ == "__main__":
    main()
