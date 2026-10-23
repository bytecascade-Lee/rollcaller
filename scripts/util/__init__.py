from common.logger import error


def fail(message: str) -> None:
    error(message)
    raise SystemExit(1)
