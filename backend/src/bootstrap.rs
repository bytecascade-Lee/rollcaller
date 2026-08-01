use crate::config::app_paths;
use crate::database::database_bootstrap;
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::io;
use std::path::Path;
use tracing::info;

/// 启动初始化：统筹整个启动流程
///
/// 执行顺序：
/// 1. 创建应用运行所需的全部目录（config / data / cache / temp / logs / webview）
/// 2. 校验数据目录可读可写（打包后的常见失败点：安装目录不可写）
/// 3. 初始化数据库连接池并执行迁移
///
/// 任一步骤失败都会返回错误，由调用方（Tauri setup 钩子）弹出原生对话框提示，
/// 避免打包后应用启动失败却无任何报错输出。
pub async fn init() -> Result<()> {
    ensure_directories()?;
    ensure_data_dir_writable()?;
    database_bootstrap::init().await?;
    Ok(())
}

/// 创建应用运行所需的全部目录
fn ensure_directories() -> Result<()> {
    let dirs = [
        app_paths::config_dir(),
        app_paths::data_dir(),
        app_paths::cache_dir(),
        app_paths::temp_dir(),
        app_paths::logs_dir(),
        app_paths::webview_dir(),
    ];
    for dir in dirs {
        fs::create_dir_all(dir)
            .with_context(|| format!("创建目录失败: {}", dir.display()))?;
        info!("目录已就绪: {}", dir.display());
    }
    Ok(())
}

/// 校验数据目录可读可写
///
/// 通过探针文件验证实际读写能力。仅检查路径存在性不足以发现权限问题，
/// 例如安装模式下应用被装到 Program Files 等不可写目录。
fn ensure_data_dir_writable() -> Result<()> {
    let probe = app_paths::data_dir().join(".write_probe");
    ensure_read_write(&probe).map_err(|e| {
        anyhow!(
            "数据目录不可读或不可写: {} ({})",
            app_paths::data_dir().display(),
            e
        )
    })?;
    let _ = fs::remove_file(&probe);
    Ok(())
}

/// 检查路径是否存在、可读、可写，不存在则创建
///
/// # 参数
/// - `path`: 要检查的路径
///
/// # 返回
/// - `Ok(())`: 路径存在且可读写
/// - `Err(io::Error)`: 出错时返回错误信息
pub fn ensure_read_write(path: &Path) -> io::Result<()> {
    // 1. 检查是否存在，不存在则创建（包括父目录）
    if !path.exists() {
        // 创建父目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // 创建文件
        fs::File::create(path)?;
    }

    // 2. 检查是否可读
    // 尝试打开文件读取
    match fs::File::open(path) {
        Ok(_) => (),
        Err(e) => return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("文件不可读: {:?}, 错误: {}", path, e)
        )),
    }

    // 3. 检查是否可写
    // 尝试以读写模式打开文件
    match fs::OpenOptions::new().write(true).open(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("文件不可写: {:?}, 错误: {}", path, e)
        )),
    }
}
