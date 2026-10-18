#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
CNB CLI 操作模块，封装常用 cnb 命令，返回结构化数据
"""

import json
import os
import re
import shutil
import subprocess
import urllib.error
import urllib.parse
import urllib.request
from functools import lru_cache
from pathlib import Path
from typing import Any

from common.logger import debug, info, warning

DEFAULT_TIMEOUT = 120
UPLOAD_TIMEOUT = 1800
# get-tag 对不存在的 tag 返回 2004002
NOT_FOUND_ERROR_CODES = frozenset({2004002})
_URL_RE = re.compile(r"https?://\S+")
_JSON_OBJECT_RE = re.compile(r"\{.*}", re.S)


class CnbError(Exception):
    """cnb 操作失败的基类"""
    pass


class CnbApiError(CnbError):
    """
    CNB API 返回 errcode（例如「标签/版本不存在」）。

    注意：这类错误不会让 cnb 进程退出码变为非 0，只有在解析响应 JSON 时才能发现。
    """

    def __init__(self, message: str, errcode: int | None = None):
        super().__init__(message)
        self.errcode = errcode


class CnbAuthError(CnbError):
    """cnb 未登录或缺少凭据"""


@lru_cache(maxsize=1)
def _cli() -> tuple[str, str]:
    """解析 cnb cli 的调用方式（node + cli 入口），见模块 docstring 第 1 条。"""
    node = os.environ.get("CNB_NODE") or shutil.which("node")
    if not node:
        raise CnbError("未找到 node（cnb cli 依赖 node，请先安装 Node.js）")

    override = os.environ.get("CNB_CLI")
    if override:
        cli_path = Path(override)
        if not cli_path.is_file():
            raise CnbError(f"CNB_CLI 指向的文件不存在: {override!r}")
        return node, str(cli_path)

    shim = shutil.which("cnb")
    if not shim:
        raise CnbError("未找到 cnb 命令（请先安装 @cnbcool/cnb-cli，或设 CNB_CLI 指向 cnb.js）")

    base = Path(shim).resolve().parent
    candidates = (
        base / "node_modules" / "@cnbcool" / "cnb-cli" / "bin" / "cnb.js",
        base / ".." / "lib" / "node_modules" / "@cnbcool" / "cnb-cli" / "bin" / "cnb.js",
    )
    for candidate in candidates:
        if candidate.is_file():
            return node, str(candidate.resolve())
    tried = "、".join(str(path) for path in candidates)
    raise CnbError(f"未找到 cnb-cli 入口（已尝试: {tried}）；可用 CNB_CLI 显式指定")


def _redact(text: str) -> str:
    """把文本里的 http(s) URL 全部替换掉（预签名地址含一次性上传凭据）"""
    return _URL_RE.sub("<redacted-url>", text)


def _run(args: list[str], *, timeout: float | int) -> subprocess.CompletedProcess:
    """
    执行 cnb 命令并返回 CompletedProcess

    :param args: cnb 子命令及其参数，如 ["auth", "status"]
    :param timeout: 超时秒数
    """
    node, cli = _cli()
    command = [node, cli, *args]
    debug(f"执行: cnb {' '.join(args)}")

    try:
        result = subprocess.run(
            command,
            capture_output=True,
            check=False,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as error:
        raise CnbError(f"cnb {' '.join(args)} 超时（{timeout}s）") from error

    if result.returncode != 0:
        stderr = _redact(result.stderr.decode("utf-8", "replace").strip())
        stdout = _redact(result.stdout.decode("utf-8", "replace").strip())
        raise CnbError(f"cnb {' '.join(args)} 失败（退出码 {result.returncode}）: {stderr or stdout}")
    return result


def stdout(args: list[str], *, timeout: float | int = DEFAULT_TIMEOUT) -> str:
    result = _run(args, timeout=timeout)
    return result.stdout.decode("utf-8", "replace").strip()


def stdout_byts(args: list[str], *, timeout: float | int = DEFAULT_TIMEOUT) -> bytes:
    return _run(args, timeout=timeout).stdout


def auth() -> None:
    """检查 cnb 登录状态"""
    try:
        stdout(["status"])
    except CnbError as e:
        raise CnbAuthError(f"cnb 未通过鉴权: {e}，请先 cnb login，或在环境中注入 CNB_TOKEN。") from e


def call(args: list[str]) -> Any:
    """
    执行 cnb 命令（附加 --verbose）并解析 JSON，剥离 `{status, ..., data}` 信封。

    Returns:
        信封中的 data；没有信封时返回 body 本身
    Raises:
        CnbApiError: data 含 errcode（标签 / 版本不存在等）
        CnbError: 命令失败或输出无法解析为 JSON
    """
    raw = stdout([*args, "--verbose"])
    try:
        body = json.loads(raw)
    except json.JSONDecodeError:
        # 容错：个别版本会在 JSON 前后夹带提示文本，取最外层 {...} 再试一次
        match = _JSON_OBJECT_RE.search(raw)
        if not match:
            raise CnbError(f"cnb {' '.join(args)} 输出无法解析为 JSON: {_redact(raw)[:500]}")
        try:
            body = json.loads(match.group(0))
        except json.JSONDecodeError as error:
            raise CnbError(f"cnb {' '.join(args)} 输出无法解析为 JSON: {_redact(raw)[:500]}") from error

    data = body.get("data") if isinstance(body, dict) else None
    if isinstance(data, dict) and "errcode" in data:
        raise CnbApiError(
            f"cnb {' '.join(args)} 失败: {_redact(str(data.get('errmsg', data['errcode'])))}",
            errcode=data.get("errcode"),
        )
    return data if data is not None else body


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
        call(["git", "get-tag", "--repo", repo, "--tag", tag])
        return True
    except CnbApiError as error:
        if error.errcode in NOT_FOUND_ERROR_CODES:
            return False
        raise


def create_tag(repo: str, tag: str, target: str) -> None:
    """创建标签，target 可为分支名 / 标签名 / 提交哈希"""
    call(["git", "create-tag", "--repo", repo, "--name", tag, "--target", target])


def release_id_by_tag(repo: str, tag: str) -> str | None:
    """
    按标签查询版本 id；该标签尚无版本时返回 None。

    Raises:
        CnbError: 鉴权失败、响应结构无法识别等
    """
    try:
        data = call(["releases", "get-release-by-tag", "--repo", repo, "--tag", tag])
    except CnbApiError as error:
        if error.errcode in NOT_FOUND_ERROR_CODES:
            return None
        raise
    if isinstance(data, dict) and data.get("id"):
        return str(data["id"])
    raise CnbError(f"cnb get-release-by-tag 响应无法识别: {_safe_payload(data)}")


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
    data = call([
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
    *,
    make_latest: bool,
    draft: bool,
    prerelease: bool,
) -> None:
    """
    更新既有版本（patch）。

    ⚠️ `--draft` / `--prerelease` 只能置真（CLI 无 `=false`），所以这里只在 True 时传参：
        草稿转正必须手动改，或改用 `--data <json>`（见模块 docstring 已知限制）。
    """
    call([
        "releases", "patch-release",
        "--repo", repo,
        "--release-id", release_id,
        "--name", name,
        "--body-file", str(body_file),
        "--make-latest", "true" if make_latest else "false",
        *(["--draft"] if draft else []),
        *(["--prerelease"] if prerelease else []),
    ])


def _redact_object(value: Any) -> Any:
    """递归脱敏：字符串里的 http(s) 地址整体替换为 ***，其余文本只替换内嵌 URL"""
    if isinstance(value, dict):
        return {key: _redact_object(item) for key, item in value.items()}
    if isinstance(value, list):
        return [_redact_object(item) for item in value]
    if isinstance(value, str):
        if value.startswith(("http://", "https://")):
            return "***"
        return _redact(value)
    return value


def _safe_payload(data: Any) -> str:
    """序列化响应用于报错文案，但抹掉其中的 URL。

    预签名地址（upload_url / verify_url）内含一次性上传凭据，泄漏到 CI 日志等于把
    上传通道交出去，故任何错误消息都不携带它们（见模块 docstring 第 3 条）。
    """
    if isinstance(data, dict):
        data = {
            key: ("***" if isinstance(value, str) and value.startswith(("http://", "https://")) else value)
            for key, value in data.items()
        }
    return json.dumps(data, ensure_ascii=False)[:300]


def _put_file(url: str, path: Path, size: int, *, timeout: float) -> None:
    """
    把文件流式 PUT 到预签名地址（不带鉴权头）。

    网络类失败重试一次：预签名 PUT 对同一 object key 是幂等的，重试不会产生脏数据。
    HTTP 4xx/5xx 不重试——那是签名过期或参数错误，重试只会拖时间。
    """
    last_error: Exception | None = None
    for attempt in (1, 2):
        try:
            with path.open("rb") as handle:
                request = urllib.request.Request(
                    url,
                    data=handle,
                    method="PUT",
                    headers={
                        "Content-Type": "application/octet-stream",
                        "Content-Length": str(size),
                    },
                )
                with urllib.request.urlopen(request, timeout=timeout):
                    return
        except urllib.error.HTTPError as error:
            detail = _redact(error.read().decode("utf-8", "replace")[:300])
            raise CnbError(f"上传附件 {path.name} 失败: HTTP {error.code}: {detail}") from error
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            last_error = error
            if attempt == 1:
                warning(f"上传附件 {path.name} 第 1 次失败（{error}），重试一次")
    raise CnbError(f"上传附件 {path.name} 失败: {last_error}")


def _confirm_upload(repo: str, release_id: str, verify_url: str) -> None:
    """从 verify_url 提取 upload_token / asset_path 并确认（URL 本身含凭据，不进日志）"""
    segments = [urllib.parse.unquote(segment) for segment in urllib.parse.urlparse(verify_url).path.split("/") if segment]
    if len(segments) < 2:
        raise CnbError(f"verify_url 无法解析，无法提取上传凭据（共 {len(segments)} 段）")
    upload_token, asset_path = segments[-2], segments[-1]
    call([
        "releases", "post-release-asset-upload-confirmation",
        "--repo", repo,
        "--release-id", release_id,
        "--upload-token", upload_token,
        "--asset-path", asset_path,
        "--ttl", "0",
    ])


def upload_release_asset(repo: str, release_id: str, path: Path, *, timeout: float = UPLOAD_TIMEOUT) -> None:
    """
    上传附件到版本：申请预签名 URL → PUT 文件 → 确认上传。

    三步缺一不可：只 PUT 不确认的附件不会出现在版本页。
    """
    size = path.stat().st_size
    data = call([
        "releases", "post-release-asset-upload-url",
        "--repo", repo,
        "--release-id", release_id,
        "--asset-name", path.name,
        "--size", str(size),
        "--overwrite",
        "--ttl", "0",
    ])
    if not isinstance(data, dict):
        raise CnbError(f"获取附件上传地址失败: {_safe_payload(data)}")
    upload_url = data.get("upload_url")
    verify_url = data.get("verify_url")
    if not upload_url or not verify_url:
        raise CnbError(f"获取附件上传地址失败: {_safe_payload(data)}")

    _put_file(str(upload_url), path, size, timeout=timeout)
    _confirm_upload(repo, release_id, str(verify_url))
    info(f"已上传 CNB 附件: {path.name} ({size} bytes)")
