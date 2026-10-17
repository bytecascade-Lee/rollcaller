from datetime import datetime, timezone


def format_to_iso_8601(value: datetime) -> str:
    """UTC、毫秒精度、Z 结尾"""
    return value.astimezone(timezone.utc).isoformat(timespec="millisecond").replace("+00:00", "Z")
