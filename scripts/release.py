#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
发布入口

LOCAL:
    推导 full 版本 -> 定位构建目录 -> 发现产物 -> 构造 latest-develop.json ->
    计算摘要 -> 构造 versions.json -> 前台驻留本地更新服务（需 Ctrl+C 手动结束）

CI:
    校验 tag -> 鉴权 -> release notes -> 发现产物 -> 构造 latest-{github|cnb}.json ->
    签名 + 计算摘要 -> 合并版本索引 -> 构造 versions.json -> GitHub / CNB 双平台串行发布

用法:
    uv run ./scripts/release.py -v 0.1.0 -m local -s normal
    uv run ./scripts/release.py -v 0.1.0 -m ci
"""

import argparse
import sys
from pathlib import Path

from build import hasher, metadata, signer
from cli import cnb as cnb_cli
from cli import gh, git, tauri
from common.constant.paths import ASSETS_DIR, PROJECT_BASE_DIR, VERSIONS_INDEX_SOURCE_PATH
from common.constant.repo import CNB_REPO, GITHUB_REPO
from common.entity.version import Version, VersionError
from common.entity.version_index import ManifestDigest
from common.enums.mode import Mode
from common.enums.severity import Severity
from common.enums.source import Source
from common.logger import error, info, warning
from release import assets as release_assets, urls, serve
from release import cnb as cnb_release
from release import github as github_release
from release import latest_manifest, local, release_notes, versions_index
from util.sigint import graceful_sigint
from util.str_format import join_repr


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


def sign_and_digest(path: Path) -> ManifestDigest:
    """对清单签名并算摘要"""
    return ManifestDigest(sha256=hasher.sha256(path).read_text(encoding="ascii").strip(),
                          signature=signer.sign(path).read_text(encoding="ascii").strip())


def release_local(version: Version, severity: Severity, rate: int) -> None:
    """LOCAL：full 版本 → 构建目录 → 清单 / 索引 → 驻留更新服务"""
    version = Version.parse(f"{version}+{metadata.build_metadata()}")
    build = local.find(version)
    info(f"本地构建目录: {build.directory}")

    notes = release_notes.default(version)
    discovered = release_assets.discover(build.directory, version, Mode.LOCAL)

    manifest = latest_manifest.build(version, notes, severity, discovered, Source.DEVELOP)
    manifest_path = latest_manifest.write(manifest, build.directory / urls.manifest_name(Source.DEVELOP))
    digest = ManifestDigest(sha256=hasher.sha256(manifest_path).read_text(encoding="ascii").strip())
    entries = versions_index.merge([], version, severity, {Source.DEVELOP: digest})
    index_path = versions_index.write(build.directory / versions_index.FILENAME, entries)
    info(f"已写入清单 {str(manifest_path)!r} 与版本索引 {str(index_path)!r}")

    serve.serve(build.directory, str(version), rate)


def release_ci(version: Version, draft: bool) -> None:
    """CI：鉴权 -> 说明 -> 发现产物 -> 双清单 -> 签名 + 摘要 -> 版本索引 -> 双平台发布"""
    try:
        gh.auth()
        cnb_cli.auth()
    except (gh.GhError, cnb_cli.CnbError) as err:
        raise ReleaseError(f"发布鉴权失败: {err}") from err

    notes = release_notes.default(version) if draft else release_notes.extract(version)
    discovered = release_assets.discover(ASSETS_DIR, version, Mode.CI)

    repo_entries = versions_index.read(VERSIONS_INDEX_SOURCE_PATH)
    # 仓库里可能有"已排期但尚未发布"的更高版本，只取不高于当前版本的
    before = [entry for entry in repo_entries if entry.version <= version]
    if len(before) < len(repo_entries):
        ahead = join_repr([str(entry.version) for entry in repo_entries if entry.version > version])
        warning(f"仓库索引含未发布的更高版本 {ahead}，已从本次上传的索引中排除")
    current = versions_index.find(before, version)
    if current is None:
        severity = Severity.default()
        warning(f"仓库版本索引未标定 {version}，按 {severity.value!r} 兜底并写入")
    else:
        severity = current.severity

    notes_path = ASSETS_DIR / "RELEASE_NOTES.md"
    notes_path.write_text(notes, encoding="utf-8", newline="\n")

    manifests = {
        source: latest_manifest.write(
            latest_manifest.build(version, notes, severity, discovered, source),
            ASSETS_DIR / urls.manifest_name(source),
        )
        for source in (Source.GITHUB, Source.CNB)
    }
    digests = {source: sign_and_digest(path) for source, path in manifests.items()}

    after = versions_index.merge(before, version, severity, digests)
    versions_index.assert_preserved(before, after, version)
    index_path = versions_index.write(ASSETS_DIR / versions_index.FILENAME, after)

    binaries = [asset.path for asset in discovered]
    sha = git.head_hash(cwd=PROJECT_BASE_DIR)
    tag = version.to_tag()

    github_release.ensure_tag(GITHUB_REPO, tag, sha)
    github_release.publish(GITHUB_REPO, version, tag, notes_path, [*binaries, manifests[Source.GITHUB], index_path],
                           draft=draft)

    cnb_release.ensure_tag(CNB_REPO, tag, sha)
    cnb_release.publish(CNB_REPO, version, tag, notes_path, [*binaries, manifests[Source.CNB], index_path], draft=draft)

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
    with graceful_sigint() as is_interrupted:
        try:
            sys.exit(main())
        except (
                ReleaseError,
                VersionError,
                git.GitError,
                local.LocalReleaseError,
                release_assets.AssetDiscoveryError,
                latest_manifest.ManifestError,
                hasher.Sha256Error,
                signer.SignError,
                tauri.TauriError,
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
            if not is_interrupted():
                error(f"发布失败: {e}")
                sys.exit(1)
