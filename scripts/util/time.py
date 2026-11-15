from datetime import UTC, datetime


def format_to_iso_8601(value: datetime) -> str:
    """UTC、毫秒精度、Z 结尾"""
    return value.astimezone(UTC).isoformat(timespec="milliseconds").replace("+00:00", "Z")
