from common.enums.os import OS


def exe(file_name: str, os: OS) -> str:
    if os == OS.WINDOWS:
        return f"{file_name}.exe"
    return file_name
