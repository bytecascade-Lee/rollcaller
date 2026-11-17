use anyhow::anyhow;
use std::fs;
use std::path::Path;

/// 分离式 spawn updater.exe
///
/// 更新器要写 `runtime.log.file`，故日志目录在这里确保存在——调用方只需给出路径。
pub(crate) fn spawn_updater(exec_path: &Path, config_path: &Path, log_file: &Path) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const DETACHED_PROCESS: u32 = 0x0000_0008;

    if let Some(parent) = log_file.parent() {
        fs::create_dir_all(parent).map_err(|e| anyhow!("创建更新日志目录失败（{}）：{e}", parent.display()))?;
    }
    let output_path = log_file.with_extension("stderr.log");
    let stdout =
        fs::File::create(&output_path).map_err(|e| anyhow!("创建更新器输出日志失败（{}）：{e}", output_path.display()))?;
    let stderr = stdout
        .try_clone()
        .map_err(|e| anyhow!("复制更新器输出日志句柄失败（{}）：{e}", output_path.display()))?;

    Command::new(exec_path)
        .arg(config_path)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS)
        .spawn()
        .map(|_| ())
        .map_err(|e| anyhow!("spawn updater.exe 失败: {e}"))
}
