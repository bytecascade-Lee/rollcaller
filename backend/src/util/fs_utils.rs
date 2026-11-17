use std::fs;
use std::io::Write;
use std::path::Path;

/// 同目录原子写：写 `<name>.tmp`（落盘后 fsync）再 rename 覆盖
///
/// 目标路径上永远不会出现"写了一半"的文件；rename 后仍是同一目录项，故不会跨卷。
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = tmp_path(path);
    let mut file = fs::File::create(&tmp)?;
    file.write_all(bytes)?;
    // 先落盘再 rename：否则断电后 rename 生效、内容却还是空的
    file.sync_all()?;
    drop(file);
    if let Err(error) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(error);
    }
    Ok(())
}

/// 原子写的临时文件路径：`<name>.tmp`
pub fn tmp_path(path: &Path) -> std::path::PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    path.with_file_name(format!("{name}.tmp"))
}
