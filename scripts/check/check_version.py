from common.entity.version import Version, VersionError


def check_version(version: str) -> Version:
    """解析发布版本号：必须可解析，且不允许携带构建元数据"""
    try:
        version = Version.parse(version)
    except ValueError as error:
        raise VersionError(f"版本号非法: {version!r}（{error}）") from error
    if version.build is not None:
        raise VersionError(f"发布版本号不允许携带构建元数据: {version}")
    return version
