#!/usr/bin/env python3
"""
本地构建打包脚本：构建 Tauri 应用并生成 setup 安装包与便携版 zip，
并产出供本地更新链路联调的 `latest-dev.json` 与 `versions.json`。

用法:
    uv run python scripts/release_local.py <版本号> [--target <target>] [--output-dir <dir>]
        [--serve-base <url>] [--severity normal|important|critical]

版本号可带 v 也可不带，例如: v0.1.0-beta.2 或 0.1.0-rc.1
与 CI 发布流程一致，但本地构建对 alpha/beta 不做限制，且产物名携带构建信息。

target 支持简化别名与 all：
    --target all                   # 打包全部支持架构（x86_64 + arm64）
    --target x64 / x86_64 / x86-64 # → x86_64-pc-windows-msvc
    --target arm64 / aarch64       # → aarch64-pc-windows-msvc
    （缺省为本机默认架构）

版本号处理：构建前自动调用 update_version.py 临时把 4 个版本文件与 uv.lock 更新为
传入版本号，构建完成后还原（不提交）。工作区对版本文件干净时用 git checkout 还原
（避免换行符问题）；存在未提交改动时回退脚本还原并给出警告。

签名：若 env/secrets/rollcaller.key 与 rollcaller.secret 非空，自动注入
TAURI_SIGNING_PRIVATE_KEY(_PASSWORD) 使 tauri bundler 用生产私钥给安装包签名
（createUpdaterArtifacts 需要；产物 .sig 与客户端内置公钥匹配，本地即可验签全链路）。
两文件未填写时构建将因缺少签名密钥而失败。

产物输出到 <output-dir>/<版本号>+<分支名>.<提交数>.<短哈希>/
    - rollcaller-<版本号>+<构建信息>-windows-<arch>-setup.exe (+ .sig)
    - rollcaller-<版本号>+<构建信息>-windows-<arch>-portable.zip
    - latest-dev.json   # 双写兼容清单；产物 URL 指向 <serve-base>/releases/download/v<版本>/...
    - versions.json     # 裸数组：扫描 release/Local 全部已构建版本，当前版本取 --severity

联调：`uv run python scripts/serve_local.py --version <版本号>` 即可本地 serve 本目录。
"""

import argparse
import json
import sys
from pathlib import Path

import update_version
from common import builder, git, manifest, packager, targets, tauri_cli, version
from common.logger import log

ROOT = Path(__file__).resolve().parent.parent
BACKEND = ROOT / "backend"
DEFAULT_OUTPUT = ROOT / "release" / "local"
# 生产签名密钥（本地构建注入 TAURI_SIGNING_PRIVATE_KEY(_PASSWORD) 用；内容不进代码）
SECRET_KEY_FILE = ROOT / "env" / "secrets" / "rollcaller.key"
SECRET_PASSWORD_FILE = ROOT / "env" / "secrets" / "rollcaller.secret"
# 仓库维护的版本索引（severity 唯一标定处；本地扫描时对同名版本沿用其 severity）
REPO_VERSIONS_INDEX = ROOT / "resources" / "update" / "versions.json"
# 本地清单文件名（serve_local 校验该文件必须存在）
LOCAL_MANIFEST_NAME = "latest-dev.json"

# update_version.py 维护的版本文件 + 锁文件；本地构建后需还原
VERSION_FILES = [
    "pyproject.toml",
    "backend/tauri.conf.json5",
    "backend/Cargo.toml",
    "backend/Cargo.lock",
    "frontend/package.json",
    "uv.lock",
]


def fail(message: str) -> None:
    log("ERROR", message)
    raise SystemExit(1)


def signing_env_overrides() -> dict:
    """从 env/secrets 读取生产签名密钥；未填写返回空 dict 并告警。

    tauri bundler（createUpdaterArtifacts）需要 TAURI_SIGNING_PRIVATE_KEY 与
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD 才会生成安装包签名 .sig。
    """
    overrides = {}
    if SECRET_KEY_FILE.is_file() and SECRET_PASSWORD_FILE.is_file():
        key = SECRET_KEY_FILE.read_text(encoding="utf-8").strip()
        password = SECRET_PASSWORD_FILE.read_text(encoding="utf-8").strip()
        if key and password:
            overrides["TAURI_SIGNING_PRIVATE_KEY"] = key
            overrides["TAURI_SIGNING_PRIVATE_KEY_PASSWORD"] = password
            log("INFO", f"已注入签名密钥（{SECRET_KEY_FILE.name} / {SECRET_PASSWORD_FILE.name}）")
        else:
            log("WARN", f"签名密钥文件为空（{SECRET_KEY_FILE}），tauri 无法生成 .sig，构建可能失败")
    else:
        log("WARN", f"缺少签名密钥文件（{SECRET_KEY_FILE} / {SECRET_PASSWORD_FILE}），"
                    f"tauri 无法生成 .sig，构建可能失败；请先填写后再构建")
    return overrides


def build_targets(
    target_list,
    release_version: str,
    full_version: str,
    out_dir: Path,
    env_overrides: dict,
) -> None:
    """对每个 target 依次构建并打包。"""
    cli_label, cli_cmd = tauri_cli.resolve(ROOT)
    for t in target_list:
        arch = packager.arch_for_target(t)
        release_dir = builder.build(
            ROOT, BACKEND, t, cli_cmd, cli_label,
            # VERSION 被 backend/build.rs 读取并嵌入二进制；签名密钥随构建注入
            env_overrides={"VERSION": release_version, **env_overrides},
        )
        setup = packager.package_setup(release_dir, full_version, arch, out_dir)
        portable = packager.package_portable(release_dir, full_version, arch, out_dir)
        for artifact in (setup, portable):
            size_mb = artifact.stat().st_size / 1024 / 1024
            log("INFO", f"已生成: {artifact} ({size_mb:.1f} MB)")


def repo_severity_map() -> dict:
    """读取仓库源 versions.json（resources/update/versions.json）→ {版本: severity}。"""
    if not REPO_VERSIONS_INDEX.is_file():
        return {}
    try:
        entries = json.loads(REPO_VERSIONS_INDEX.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return {}
    if not isinstance(entries, list):
        return {}
    return {
        e["version"]: e.get("severity", "normal")
        for e in entries if isinstance(e, dict) and e.get("version")
    }


def scan_local_versions(local_dir: Path) -> dict:
    """扫描 release/Local 全部已构建版本目录 → {核心版本: severity}。

    目录命名 `<core>+<branch>.<count>.<hash>`；core 为第一个 '+' 之前的部分。
    同一 core 多个目录只保留一个（severity 取仓库源标定，未标定 normal）。
    """
    repo_sev = repo_severity_map()
    found = {}
    if not local_dir.is_dir():
        return found
    for d in local_dir.iterdir():
        if not d.is_dir() or "+" not in d.name:
            continue
        core = d.name.split("+", 1)[0]
        try:
            version.validate(core)
        except version.VersionError:
            continue
        found[core] = repo_sev.get(core, "normal")
    return found


def write_latest_manifest(out_dir: Path, release_version: str, full_version: str,
                          serve_base: str, severity: str) -> Path:
    """在产物目录生成双写兼容 latest-dev.json；URL 指向 serve_base 同构路径。"""
    payloads, legacy_sigs = {}, {}
    for arch in ("x86_64", "arm64"):
        setup = out_dir / packager.asset_name(full_version, arch, "setup", "exe")
        if not setup.is_file():
            continue
        sig_raw = manifest.read_sig_text(setup.with_suffix(".exe.sig"))
        legacy_sigs[arch] = sig_raw
        url = f"{serve_base}/releases/download/v{release_version}/{setup.name}"
        payloads[arch] = {"nsis": manifest.build_artifact(url, setup, sig_raw)}
        portable = out_dir / packager.asset_name(full_version, arch, "portable", "zip")
        if portable.is_file():
            purl = f"{serve_base}/releases/download/v{release_version}/{portable.name}"
            payloads[arch]["portable"] = manifest.build_artifact(purl, portable, "")
    if not payloads:
        fail(f"产物目录 {out_dir} 下未找到任何 setup 安装包，无法生成清单")

    notes = f"本地构建 {full_version}（本地更新链路联调用）"
    latest = manifest.build_latest_json(
        version=release_version,
        notes=notes,
        severity=severity,
        payloads=payloads,
        legacy_sigs=legacy_sigs,
    )
    target = out_dir / LOCAL_MANIFEST_NAME
    target.write_text(json.dumps(latest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    log("INFO", f"已生成 {target.relative_to(ROOT).as_posix()}（serve_base={serve_base}）")
    return target


def write_versions_index(out_dir: Path, release_version: str, severity: str) -> Path:
    """生成裸数组 versions.json：release/Local 全部已构建版本，当前版本取 --severity。

    写入当前产物目录；serve_local 加载该版本目录时直接 serve 此文件。
    """
    found = scan_local_versions(out_dir.parent)
    found[release_version] = severity
    entries = [
        {"version": ver, "severity": sev}
        for ver, sev in sorted(
            found.items(),
            key=lambda kv: version.parse(kv[0])[:3],
            reverse=True,
        )
    ]
    target = out_dir / "versions.json"
    target.write_text(json.dumps(entries, ensure_ascii=False, indent=4) + "\n", encoding="utf-8")
    log("INFO", f"已生成 {target.relative_to(ROOT).as_posix()}（{len(entries)} 个版本条目）")
    return target


def main() -> None:
    if not sys.platform.startswith("win"):
        fail(f"不支持当前操作系统: {sys.platform}，本地构建仅支持 Windows")

    parser = argparse.ArgumentParser(
        description="本地构建 Tauri 应用并打包 setup.exe / 便携版 zip，"
                    "同时生成 latest-dev.json 与 versions.json（本地更新链路联调用）"
    )
    parser.add_argument(
        "version",
        help="版本号，可带 v 也可不带，例如 v0.1.0-beta.2 或 0.1.0-rc.1",
    )
    parser.add_argument(
        "--target",
        default=None,
        help="架构（别名/all/完整三元组），如 x64、arm64、all；缺省为本机默认",
    )
    parser.add_argument(
        "--output-dir",
        default=str(DEFAULT_OUTPUT),
        help="产物根目录（内部按 full_version 分子目录），默认 release/local",
    )
    parser.add_argument(
        "--serve-base",
        default="http://127.0.0.1:14652",
        help="清单中产物 URL 的服务器 base（与 serve_local 默认端口一致）",
    )
    parser.add_argument(
        "--severity",
        choices=manifest.SEVERITY_LEVELS,
        default="normal",
        help="当前版本的严重级别（默认 normal；critical 表示强制更新）",
    )
    args = parser.parse_args()

    try:
        release_version = version.validate(args.version, min_level=None)
    except version.VersionError as e:
        fail(str(e))
    if "+" in release_version:
        fail(
            f"本地构建会在版本号后追加构建信息，不允许版本号自带 + 构建元数据: {args.version!r}"
        )

    try:
        target_list = targets.targets_for(args.target)
    except targets.TargetError as e:
        fail(str(e))

    build_info = git.get_build_info(cwd=ROOT)
    full_version = f"{release_version}+{build_info}"
    out_dir = Path(args.output_dir) / full_version
    log("INFO", f"版本号: {release_version} | 构建信息: {build_info}")
    log("INFO", f"目标架构: {', '.join(t or '(本机默认)' for t in target_list)}")
    log("INFO", f"产物目录: {out_dir}")

    version_files = [ROOT / p for p in VERSION_FILES]
    clean = git.are_clean(version_files, cwd=ROOT)
    if not clean[0]:
        fail(
            f"版本文件{clean[1].replace('\n', '、')}存在未提交改动，请先提交或清理后再构建，"
            "避免构建后 git 还原误伤你的改动"
        )

    env_overrides = signing_env_overrides()
    try:
        # 构建前统一由 update_version.py 更新版本号（不提交）
        update_version.sync(release_version)
        build_targets(target_list, release_version, full_version, out_dir, env_overrides)
        # 打包后生成联调清单与版本索引（产物 URL 指向 serve_base）
        write_latest_manifest(out_dir, release_version, full_version, args.serve_base, args.severity)
        write_versions_index(out_dir, release_version, args.severity)
    finally:
        git.restore_files(version_files, cwd=ROOT)
        log("INFO", "已用 git 还原版本号文件（未提交）")


if __name__ == "__main__":
    main()
