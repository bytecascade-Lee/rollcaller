#!/usr/bin/env python3
"""
CNB（cnb.cool）CLI 操作模块：封装常用 cnb 命令，返回结构化数据。

与 git.py / gh.py 保持相同约定：
- 所有函数不做环境嗅探、不探测默认仓库（仓库以 "owner/repo" 显式传入），
- 失败时抛出异常，由调用方决定如何处理。

两条已内收的坑：
1. Windows 上 npm 安装的 cnb 是 .cmd shim，无法被 subprocess 直接执行，且 cmd.exe /c
   会拆解含空格/引号的参数 ⇒ 统一改为 node 直接调用 cli 入口。
2. **CNB 的 API 错误不会让进程退出码变为非 0**（实测 get-tag 对不存在的 tag 返回
   errcode=2004002 而 exit code 仍为 0）⇒ 成败只能看响应 JSON 的 errcode，本模块据此
   抛出 CnbApiError，不能依赖 subprocess 的 returncode。

响应信封：cnb cli --verbose 输出 {status, trace, header, contentType, data}，载荷在 data
中；出错时 data 形如 {errcode, errmsg}。cnb_json() 负责剥壳并把 errcode 转成异常。

目前覆盖的用法：
- tag_exists() / create_tag(): 标签查询与创建
- release_id_by_tag() / create_release() / update_release(): 版本查询与创建/更新
- upload_release_asset(): 附件三步上传（申请预签名 URL → PUT → 确认）
"""

import json
import re
import shutil
import subprocess
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any, List, Optional

from common.logger import log


class CnbError(Exception):
    """cnb 操作失败的基类"""
    pass


class CnbApiError(CnbError):
    """
    CNB API 返回 errcode（例如「标签/版本不存在」）。

    注意：这类错误不会让 cnb 进程退出码变为非 0，只有在解析响应 JSON 时才能发现。
    """

    def __init__(self, message: str, errcode: Optional[int] = None):
        super().__init__(message)
        self.errcode = errcode


def _cli() -> List[str]:
    """解析 cnb cli 的调用方式（node + cli 入口），见模块 docstring 第 1 条。"""
    node = shutil.which("node")
    if not node:
        raise CnbError("未找到 node（cnb cli 依赖 node，请先安装 Node.js）")
    shim = shutil.which("cnb")
    if not shim:
        raise CnbError("未找到 cnb 命令（请先安装 @cnbcool/cnb-cli）")
    cli = Path(shim).resolve().parent / "node_modules" / "@cnbcool" / "cnb-cli" / "bin" / "cnb.js"
    if not cli.exists():
        raise CnbError(f"未找到 cnb-cli 入口: {cli}")
    return [node, str(cli)]


def cnb(args: List[str]) -> str:
    """
    执行 cnb 命令，返回 stdout 字符串（utf-8）。

    Args:
        args: cnb 子命令参数，如 ["status"]、["git", "get-tag", "--repo", "..."]
    Raises:
        CnbError: cnb 不可用，或命令以非零退出码结束
    """
    cmd = [*_cli(), *args]
    log("INFO", f"执行: cnb {' '.join(args)}")
    proc = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    if proc.returncode != 0:
        raise CnbError(f"cnb {' '.join(args)} 失败: {proc.stderr.strip() or proc.stdout.strip()}")
    return proc.stdout.strip()


def cnb_json(args: List[str]) -> Any:
    """
    执行 cnb 命令（附加 --verbose）并解析 JSON，剥离 {status, ..., data} 信封。

    Args:
        args: cnb 子命令参数（不必自带 --verbose）
    Returns:
        信封中 data 字段的内容
    Raises:
        CnbApiError: 响应 data 含 errcode（如标签/版本不存在）
        CnbError: 命令执行失败或响应无法解析为 JSON
    """
    proc = cnb([*args, "--verbose"])
    text = proc.strip()
    body = None
    try:
        body = json.loads(text)
    except json.JSONDecodeError:
        # 容错：个别版本会在 JSON 前后夹带提示文本，取最外层 {...} 再试一次
        match = re.search(r"\{.*\}", text, re.S)
        if match:
            try:
                body = json.loads(match.group(0))
            except json.JSONDecodeError:
                body = None
    if body is None:
        raise CnbError(f"cnb {' '.join(args)} 输出无法解析为 JSON: {text[:500]}")

    data = body.get("data") if isinstance(body, dict) else None
    if isinstance(data, dict) and "errcode" in data:
        raise CnbApiError(
            f"cnb {' '.join(args)} 失败: {data.get('errmsg', data['errcode'])}",
            errcode=data.get("errcode"),
        )
    # 剥离 {status, ..., data} 包装（成功响应）
    if data is not None:
        return data
    return body


def tag_exists(repo: str, tag: str) -> bool:
    """
    探测仓库中是否已有该标签。

    探测场景下把一切失败都视为「不存在」：如标签确实不存在（API errcode），
    以及 cnb 不可用、网络异常等——后者交给创建标签的报错路径给出诊断，避免在轮询中
    因一次抖动直接中断发布。

    Args:
        repo: "owner/repo"
        tag: 标签名，如 "v0.8.0"
    """
    try:
        cnb_json(["git", "get-tag", "--repo", repo, "--tag", tag])
        return True
    except CnbError:
        return False


def create_tag(repo: str, tag: str, target: str) -> None:
    """
    创建标签。

    Args:
        repo: "owner/repo"
        tag: 标签名，如 "v0.8.0"
        target: 标签指向的目标，可为分支名 / 标签名 / 提交哈希
    Raises:
        CnbApiError: 标签已存在等 API 层错误
        CnbError: 命令执行失败
    """
    cnb_json(["git", "create-tag", "--repo", repo, "--name", tag, "--target", target])


def release_id_by_tag(repo: str, tag: str) -> Optional[str]:
    """
    按标签查询版本 id。

    Args:
        repo: "owner/repo"
        tag: 标签名，如 "v0.8.0"
    Returns:
        版本 id 字符串；该标签尚无版本时返回 None
    Raises:
        CnbError: 响应结构无法识别
    """
    try:
        data = cnb_json(["releases", "get-release-by-tag", "--repo", repo, "--tag", tag])
    except CnbApiError:
        return None
    if isinstance(data, dict) and data.get("id"):
        return str(data["id"])
    raise CnbError(f"cnb get-release-by-tag 响应无法识别: {json.dumps(data, ensure_ascii=False)[:300]}")


def create_release(
    repo: str,
    tag: str,
    name: str,
    body_file: Path,
    make_latest: bool = True,
    draft: bool = False,
    prerelease: bool = False,
) -> str:
    """
    创建版本（post），返回新版本 id。

    Args:
        repo: "owner/repo"
        tag: 标签名（该标签须已存在，否则 CNB 无法挂载版本）
        name: 版本标题
        body_file: 版本说明文件
        make_latest: 是否置为最新版本
        draft: 是否为草稿
        prerelease: 是否为预发布
    Raises:
        CnbError: 创建失败，或成功但未返回 id
    """
    data = cnb_json([
        "releases", "post-release",
        "--repo", repo,
        "--tag-name", tag,
        "--name", name,
        "--body-file", str(body_file),
        "--make-latest", "true" if make_latest else "false",
        *(["--draft"] if draft else []),
        *(["--prerelease"] if prerelease else []),
    ])
    release_id = data.get("id") if isinstance(data, dict) else None
    if not release_id:
        raise CnbError(f"创建 CNB Release 后未获取到 id: {json.dumps(data, ensure_ascii=False)[:300]}")
    return str(release_id)


def update_release(
    repo: str,
    release_id: str,
    name: str,
    body_file: Path,
    make_latest: bool = True,
    draft: bool = False,
    prerelease: bool = False,
) -> None:
    """
    更新既有版本（patch）。

    Args:
        repo: "owner/repo"
        release_id: 版本 id（release_id_by_tag / create_release 的返回值）
        其余参数同 create_release
    """
    cnb_json([
        "releases", "patch-release",
        "--repo", repo,
        "--release-id", release_id,
        "--name", name,
        "--body-file", str(body_file),
        "--make-latest", "true" if make_latest else "false",
        *(["--draft"] if draft else []),
        *(["--prerelease"] if prerelease else []),
    ])


def upload_release_asset(repo: str, release_id: str, path: Path) -> None:
    """
    上传附件到版本：申请预签名 URL → PUT 文件 → 确认上传。

    预签名 URL 不带鉴权头直接 PUT，故文件内容不经 cnb cli；三步缺一不可，
    只 PUT 不确认的附件不会出现在版本页。

    Args:
        repo: "owner/repo"
        release_id: 版本 id
        path: 本地文件路径
    Raises:
        CnbError: 申请地址失败、PUT 失败或确认失败
    """
    size = path.stat().st_size
    data = cnb_json([
        "releases", "post-release-asset-upload-url",
        "--repo", repo,
        "--release-id", release_id,
        "--asset-name", path.name,
        "--size", str(size),
        "--overwrite",
        "--ttl", "0",
    ])
    if not isinstance(data, dict):
        raise CnbError(f"获取附件上传地址失败: {json.dumps(data, ensure_ascii=False)[:300]}")
    upload_url = data.get("upload_url")
    verify_url = data.get("verify_url")
    if not upload_url or not verify_url:
        raise CnbError(f"获取附件上传地址失败: {json.dumps(data, ensure_ascii=False)[:300]}")

    # 预签名 URL：不带鉴权头直接 PUT 文件内容
    req = urllib.request.Request(
        upload_url,
        data=path.read_bytes(),
        method="PUT",
        headers={"Content-Type": "application/octet-stream"},
    )
    try:
        with urllib.request.urlopen(req, timeout=600):
            pass
    except urllib.error.HTTPError as e:
        detail = e.read().decode("utf-8", errors="replace")[:300]
        raise CnbError(f"上传附件 {path.name} 失败: HTTP {e.code}: {detail}")
    except urllib.error.URLError as e:
        raise CnbError(f"上传附件 {path.name} 失败: {e.reason}")

    # 从 verify_url 提取 upload_token / asset_path 并确认
    segments = [urllib.parse.unquote(s) for s in urllib.parse.urlparse(verify_url).path.split("/") if s]
    if len(segments) < 2:
        raise CnbError(f"verify_url 无法解析: {verify_url}")
    upload_token, asset_path = segments[-2], segments[-1]
    cnb_json([
        "releases", "post-release-asset-upload-confirmation",
        "--repo", repo,
        "--release-id", release_id,
        "--upload-token", upload_token,
        "--asset-path", asset_path,
        "--ttl", "0",
    ])
    log("INFO", f"已上传 CNB 附件: {path.name} ({size} bytes)")
