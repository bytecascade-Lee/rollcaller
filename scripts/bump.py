#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
发布 tag 入口

按档位推算下一个版本号，在 master 上打附注 tag。不动任何版本文件——
版本号由 `scripts/build.py` 在构建期临时写入，构建结束即还原。

档位：

| 档位         | 0.8.2 稳定版        | 0.9.0-rc.2 预发布态  |
|:-------------|:--------------------|:---------------------|
| `major`      | 1.0.0               | 1.0.0                |
| `minor`      | 0.9.0               | 0.9.0（毕业）        |
| `patch`      | 0.8.3               | 0.9.0（毕业）        |
| `prerelease` | 0.8.3-alpha.1       | rc 已是最后一档      |

- `prerelease`：稳定版进入 `alpha.1`；已有预发布段则换下一档并重置序号（`alpha.3` → `beta.1`）
- `prerelease --serial`：同档序号 +1（`alpha.1` → `alpha.2`）；`rc` 已是最后一档
- 从稳定版进入预发布时，基线默认取下一个 patch；`--base minor` 可改成下一个 minor

用法:
    uv run ./scripts/bump.py patch
    uv run ./scripts/bump.py prerelease --base minor
    uv run ./scripts/bump.py prerelease --serial
"""

import argparse
import re
import sys

from cli import git
from common.entity.version import Version, VersionError
from common.logger import error, info, success, warning
from util.sigint import graceful_sigint

# 远端名：只用于核对 master 是否已推送，本脚本不推送任何东西
REMOTE = "github"

# 预发布档位，顺序即推进方向
STAGES = ("alpha", "beta", "rc")

# 预发布段形如 alpha.1 / beta.2 / rc.3
_PRERELEASE_RE = re.compile(r"^(alpha|beta|rc)\.(\d+)$")


class BumpError(Exception):
    """推算下一个版本号或打 tag 的前置条件不满足"""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="按档位推算下一个版本号，并在 master 上打发布 tag")
    parser.add_argument("kind", choices=("major", "minor", "patch", "prerelease"), help="版本档位")
    parser.add_argument("--serial", action="store_true", help="prerelease 档内序号 +1，如 alpha.1 -> alpha.2")
    parser.add_argument("--base", choices=("major", "minor", "patch"), default="patch",
                        help="稳定版进入预发布时的基线，默认 patch")
    parser.add_argument("--dry-run", action="store_true", help="只打印将要打的 tag，不创建")

    args = parser.parse_args()
    if args.serial and args.kind != "prerelease":
        parser.error("--serial 只能与 prerelease 档一起使用")
    return args


def main() -> int:
    args = parse_args()

    _ensure_on_master()
    _ensure_worktree_clean()
    _ensure_master_in_sync()

    current = _latest_version()
    target = _next_version(current, args)
    tag = target.to_tag()
    _ensure_tag_absent(tag)

    info(f"最新版本 {current.to_tag()}")
    success(f"下一个版本 {tag}")

    if args.dry_run:
        info("--dry-run：未创建 tag")
        return 0

    git.create_tag(tag, f"发布版本 {tag}")
    success(f"已在 master 上创建 tag {tag}")
    info(f"本脚本不推送；需要发布时自行执行：git push {REMOTE} {tag}")
    return 0


def _ensure_on_master() -> None:
    """发布 tag 只能在 master 上打（它标记的是生产线的提交）"""
    branch = git.current_branch()
    if branch != "master":
        raise BumpError(f"当前分支是 {branch or 'detached HEAD'}，发布 tag 只能在 master 上打")


def _ensure_worktree_clean() -> None:
    """已跟踪文件必须无改动；未跟踪文件不影响（tag 只记 HEAD）"""
    changes = git.tracked_changes()
    if changes.strip():
        count = len([line for line in changes.splitlines() if line.strip()])
        raise BumpError(f"有 {count} 个已跟踪文件改动未提交，先提交再打 tag")


def _ensure_master_in_sync() -> None:
    """本地 master 必须与远端一致，否则 tag 会指向一个尚未推送的提交"""
    remote = git.try_rev_parse(f"{REMOTE}/master")
    if remote is None:
        warning(f"找不到 {REMOTE}/master，跳过与远端的一致性检查，可先 git fetch {REMOTE}")
        return

    head = git.head_hash()
    if remote != head:
        raise BumpError(f"本地 master {head[git.ABBREVIATED_HASH_SLICE]} 与 "
                        f"{REMOTE}/master {remote[git.ABBREVIATED_HASH_SLICE]} 不一致，请先同步")


def _ensure_tag_absent(tag: str) -> None:
    """版本号不复用：同名 tag 已存在就停下"""
    if git.try_rev_parse(f"refs/tags/{tag}") is not None:
        raise BumpError(f"tag {tag} 已存在；版本号不复用，请换一档或人工处理")


def _latest_version() -> Version:
    """取仓库里最大的版本 tag；无法解析的 tag 跳过并告警"""
    versions = []
    for name in git.tags("v*"):
        try:
            versions.append(Version.parse(name))
        except VersionError:
            warning(f"跳过无法解析的 tag {name!r}")
    if not versions:
        raise BumpError("仓库里没有可解析的版本 tag，无法推算下一个版本")
    return max(versions)


def _next_version(current: Version, args: argparse.Namespace) -> Version:
    if args.kind == "major":
        return _next_major(current)
    if args.kind == "minor":
        return _next_minor(current)
    if args.kind == "patch":
        return _next_patch(current)
    if args.serial:
        return _next_prerelease_serial(current)
    if current.prerelease is not None and args.base != "patch":
        warning(f"当前已是预发布版本 {current}，--base 忽略")
    return _next_prerelease_stage(current, args.base)


def _next_major(current: Version) -> Version:
    """预发布挂在 X.0.0 上时，major 档表示毕业到 X.0.0 正式版"""
    if current.prerelease is not None and current.minor == 0 and current.patch == 0:
        return _stable(current)
    return Version.parse(f"{current.major + 1}.0.0")


def _next_minor(current: Version) -> Version:
    """预发布挂在 X.Y.0 上时，minor 档表示毕业到 X.Y.0 正式版"""
    if current.prerelease is not None and current.patch == 0:
        return _stable(current)
    return Version.parse(f"{current.major}.{current.minor + 1}.0")


def _next_patch(current: Version) -> Version:
    """预发布态一律毕业到它的基线版本（0.8.3-rc.2 -> 0.8.3）"""
    if current.prerelease is not None:
        return _stable(current)
    return Version.parse(f"{current.major}.{current.minor}.{current.patch + 1}")


def _next_prerelease_stage(current: Version, base: str) -> Version:
    """换档：稳定版进 alpha.1；alpha→beta、beta→rc 并重置序号；rc 是最后一档"""
    if current.prerelease is None:
        bumpers = {"major": _next_major, "minor": _next_minor, "patch": _next_patch}
        return _prerelease(bumpers[base](current), STAGES[0], 1)

    stage, _ = _split_prerelease(current)
    index = STAGES.index(stage)
    if index + 1 >= len(STAGES):
        raise BumpError(f"{current} 已在最后一档 {stage}，正式发布请用 major / minor / patch 档")
    return _prerelease(_stable(current), STAGES[index + 1], 1)


def _next_prerelease_serial(current: Version) -> Version:
    """进号：alpha.1 -> alpha.2，档位不变"""
    if current.prerelease is None:
        raise BumpError(f"{current} 是稳定版，先用 prerelease 档进入 alpha")
    stage, number = _split_prerelease(current)
    return _prerelease(_stable(current), stage, number + 1)


def _split_prerelease(version: Version) -> tuple[str, int]:
    """把预发布段拆成 (档位, 序号)；不认识的形态直接报错，避免瞎猜"""
    match = _PRERELEASE_RE.match(version.prerelease or "")
    if match is None:
        raise BumpError(f"{version} 的预发布段 {version.prerelease!r} 不是 alpha.N / beta.N / rc.N")
    stage, number = match.groups()
    return stage, int(number)


def _stable(version: Version) -> Version:
    """去掉预发布段，保留 X.Y.Z"""
    return Version.parse(f"{version.major}.{version.minor}.{version.patch}")


def _prerelease(base: Version, stage: str, number: int) -> Version:
    return Version.parse(f"{base}-{stage}.{number}")


if __name__ == "__main__":
    with graceful_sigint() as is_interrupted:
        try:
            sys.exit(main())
        except (BumpError, VersionError, git.GitError) as e:
            if not is_interrupted():
                error(f"打 tag 失败: {e}")
                sys.exit(1)
