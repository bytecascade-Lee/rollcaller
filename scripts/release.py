#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
发布入口

LOCAL:
    推导 full 版本 -> 定位构建目录 -> 发现产物 -> 构造 versions.json ->
    构造 latest-develop.json -> 前台驻留本地更新服务（需 Ctrl+C 手动结束）

CI:
    校验 tag -> 鉴权 -> release notes -> 发现产物 -> 构造 versions.json ->
    构造 latest-{github|cnb}.json -> GitHub / CNB 双平台串行发布

用法:
    uv run ./scripts/release.py -v 0.1.0 -m local -s normal
    uv run ./scripts/release.py -v 0.1.0 -m ci
"""

import argparse
import sys

from build import metadata
from cli import cnb as cnb_cli
from cli import gh, git
from common.constant.paths import ASSETS_DIR, PROJECT_BASE_DIR, VERSIONS_INDEX_SOURCE_PATH
from common.constant.repo import CNB_REPO, GITHUB_REPO
from common.entity.version import Version, VersionError
from common.entity.version_index import VersionIndex
from common.enums.mode import Mode
from common.enums.severity import Severity
from common.enums.source import Source
from common.logger import error, info, warning
from release import assets as release_assets, urls, serve
from release import cnb as cnb_release
from release import github as github_release
from release import latest_manifest, local, release_notes, versions_index
from util.sigint import graceful_sigint


class ReleaseError(Exception):
    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="发布入口")

    parser.add_argument("-v", "--version", required=True, metavar="VERSION", help="语义化版本号")
    parser.add_argument("-m", "--mode", required=True, choices=Mode.values(), help="发布模式")
    parser.add_argument("-s", "--severity", choices=Severity.values(), help="严重级别")
    parser.add_argument("--draft", action="store_true", help="是否草稿模式发布")
    parser.add_argument("-r", "--rate", type=int, default=serve.DEFAULT_RATE_KB, metavar="KBPS",
                        help="本地更新服务限速 KB/s")

    return parser.parse_args()


def release_local(version: Version, severity: Severity, rate: int) -> None:
    """LOCAL：full 版本 → 构建目录 → 清单 / 索引 → 驻留更新服务"""
    version = Version.parse(f"{version}+{metadata.build_metadata()}")
    build = local.find(version)
    info(f"本地构建目录: {build.directory}")

    notes = release_notes.default(version)
    discovered = release_assets.discover(build.directory, version, Mode.LOCAL)

    manifest = latest_manifest.build(version, notes, severity, discovered, Source.DEVELOP)
    manifest_path = latest_manifest.write(manifest, build.directory / urls.manifest_name(Source.DEVELOP))
    index_path = versions_index.write(build.directory / versions_index.FILENAME, [VersionIndex(version, severity)])
    info(f"已写入清单 {str(manifest_path)!r} 与版本索引 {str(index_path)!r}")

    serve.serve(build.directory, str(version), rate)


def release_ci(version: Version, draft: bool) -> None:
    """CI：鉴权 → 说明 → 发现产物 → 版本索引 → 双清单 → 双平台发布"""
    try:
        gh.auth()
        cnb_cli.auth()
    except (gh.GhError, cnb_cli.CnbError) as error:
        raise ReleaseError(f"发布鉴权失败: {error}") from error

    notes = release_notes.default(version) if draft else release_notes.extract(version)
    discovered = release_assets.discover(ASSETS_DIR, version, Mode.CI)

    entries = versions_index.read(VERSIONS_INDEX_SOURCE_PATH)
    current = versions_index.find(entries, version)
    if current is None:
        severity = Severity.default()
        warning(f"仓库版本索引未标定 {version}，按 {severity.value!r} 兜底并写入")
        entries = versions_index.merge(entries, version, severity)
    else:
        severity = current.severity
    index_path = versions_index.write(ASSETS_DIR / versions_index.FILENAME, entries)

    notes_path = ASSETS_DIR / "RELEASE_NOTES.md"
    notes_path.write_text(notes, encoding="utf-8")

    manifests = []
    for source in (Source.GITHUB, Source.CNB):
        manifest = latest_manifest.build(version, notes, severity, discovered, source)
        manifests.append(latest_manifest.write(manifest, ASSETS_DIR / urls.manifest_name(source)))

    # 上传顺序为 二进制 -> 清单 -> 版本索引，.sig/.sha256 只经 artifact 传递
    files = [asset.path for asset in discovered] + manifests + [index_path]
    sha = git.head_hash(cwd=PROJECT_BASE_DIR)
    tag = version.to_tag()

    github_release.ensure_tag(GITHUB_REPO, tag, sha)
    github_release.publish(GITHUB_REPO, version, tag, notes_path, files, draft=draft)

    cnb_release.ensure_tag(CNB_REPO, tag, sha)
    cnb_release.publish(CNB_REPO, version, tag, notes_path, files, draft=draft)

    info(f"发布完成: {tag}")


def main() -> int:
    args = parse_args()
    version = Version.parse(args.version, erase_build_metadata=True)
    mode = Mode(args.mode)

    if mode == Mode.LOCAL:
        if args.draft:
            warning("--draft 仅用于 CI，LOCAL 忽略")
        severity = Severity(args.severity) if args.severity is not None else Severity.default()
        release_local(version, severity, args.rate)
        return 0

    if args.severity is not None:
        warning("-s/--severity 仅用于 LOCAL，CI 的 severity 来自仓库版本索引")
    if args.rate != serve.DEFAULT_RATE_KB:
        warning("-r/--rate 仅用于 LOCAL")

    release_ci(version, args.draft)
    return 0


if __name__ == "__main__":
    with graceful_sigint():
        try:
            sys.exit(main())
        except (
                ReleaseError,
                VersionError,
                git.GitError,
                local.LocalReleaseError,
                release_assets.AssetDiscoveryError,
                latest_manifest.ManifestError,
                serve.ServeError,
                release_notes.ReleaseNotesNotFound,
                release_notes.ReleaseNotesAmbiguous,
                release_notes.ReleaseNotesSectionEmpty,
                versions_index.VersionIndexError,
                github_release.GithubReleaseError,
                cnb_release.CnbReleaseError,
                gh.GhError,
                cnb_cli.CnbError,
        ) as e:
            error(f"发布失败: {e}")
            sys.exit(1)
