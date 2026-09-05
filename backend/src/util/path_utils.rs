use std::path::{Path, PathBuf};

/// Windows 路径 → 正斜杠（`C:/...`），供 JSON 写入
#[cfg(target_os = "windows")]
pub fn to_slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// 当前可执行文件路径（去掉 Windows `\\?\` 长路径前缀，得到 `C:/...` 形式）
#[cfg(target_os = "windows")]
pub(crate) fn current_exe_clean() -> Result<PathBuf, std::io::Error> {
    let raw = std::env::current_exe()?;
    let s = raw.to_string_lossy();
    let cleaned = s.strip_prefix(r"\\?\").unwrap_or(&s);
    Ok(PathBuf::from(cleaned))
}
