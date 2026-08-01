use directories::ProjectDirs;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tracing::info;
use ts_rs::TS;

/// 应用信息常量
const APP_QUALIFIER: &str = "com";
const APP_ORG: &str = "serene";
const APP_NAME: &str = "rollcaller";

/// 模式枚举
#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[ts(export)]
pub enum AppMode {
    Develop,  // 开发模式: ./data
    Portable, // 便携模式: exe_dir/data
    Install,  // 安装模式: 系统规范路径
}

/// 路径结构体
#[derive(Debug)]
struct AppPaths {
    mode: AppMode,
    config_dir: PathBuf,
    data_dir: PathBuf,
    cache_dir: PathBuf,
    temp_dir: PathBuf,
    logs_dir: PathBuf,
    webview_dir: PathBuf,
    resources_dir: PathBuf,
}

/// 懒加载单例
static PATHS: LazyLock<AppPaths> = LazyLock::new(|| {
    let mode = detect_mode();
    let base_dir = detect_base_dir(mode);
    info!("current mode:{:#?}", &mode);
    info!("base dir:{:?}", &base_dir);
    AppPaths {
        mode,
        config_dir: base_dir.join("config"),
        data_dir: base_dir.join("data"),
        cache_dir: base_dir.join("cache"),
        temp_dir: base_dir.join("temp"),
        logs_dir: base_dir.join("logs"),
        webview_dir: base_dir.join("cache/webview2"),
        resources_dir: detect_resources_dir(mode),
    }
});

/// 模式检测
fn detect_mode() -> AppMode {
    // 开发模式: debug 构建自动启用
    if cfg!(debug_assertions) {
        return AppMode::Develop;
    }

    // 便携模式: 检测 .portable 文件
    if let Some(exe_dir) = current_exe_dir() {
        if exe_dir.join(".portable").is_file() {
            return AppMode::Portable;
        }
    }

    // 安装模式: 使用 directories 库获取各平台规范路径
    AppMode::Install
}

/// 获取基础目录
fn detect_base_dir(mode: AppMode) -> PathBuf {
    match mode {
        // 项目根目录下的 data
        // 此处不能使用 current_exe_dir
        // 因为编译出的二进制文件并不在项目根目录下面
        AppMode::Develop => dev_dir().join("data"),

        // 可执行文件目录下的 data
        AppMode::Portable => current_exe_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("data"),
        // 使用 directories 库获取各平台规范路径
        // 失败时回退到当前目录
        AppMode::Install => ProjectDirs::from(APP_QUALIFIER, APP_ORG, APP_NAME)
            .map(|proj_dirs| proj_dirs.data_local_dir().parent().unwrap().to_path_buf())
            .unwrap_or_else(|| {
                eprintln!("警告: 无法获取应用目录，使用当前目录作为回退");
                PathBuf::from(".")
            }),
    }
}

fn detect_resources_dir(mode: AppMode) -> PathBuf {
    #[cfg(target_os = "windows")]
    match mode {
        AppMode::Develop => dev_dir().join("resources"),
        // 便携/安装模式下，Tauri 的资源目录就是可执行文件所在目录
        // （tauri-utils platform::resource_dir 在 Windows 上返回 exe 所在目录，
        //  打包时 bundle.resources 的 "../resources/xxx/" 会被复制到 exe 旁边）
        AppMode::Portable | AppMode::Install => current_exe_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .to_path_buf(),
    }
    #[cfg(target_os = "macos")]
    match mode {
        AppMode::Develop => dev_dir().join("resources"),
        AppMode::Portable | AppMode::Install => current_exe_dir()
            .map(|p| p.join("../Resources"))
            .unwrap_or_else(|| PathBuf::from("."))
            .to_path_buf(),
    }
    #[cfg(target_os = "linux")]
    match mode {
        AppMode::Develop => dev_dir().join("resources"),
        // [TODO]: 随后处理
        AppMode::Portable | AppMode::Install => PathBuf::from("."),
    }
}

/// 开发模式下资源文件的路径
fn dev_dir() -> PathBuf {
    match std::env::current_dir() {
        Ok(cwd) => match cwd.parent() {
            Some(parent) => parent.to_path_buf(),
            None => {
                eprintln!("警告: 当前工作目录 {} 没有父目录，使用当前目录作为开发目录", cwd.display());
                PathBuf::from(".")
            }
        },
        Err(e) => {
            eprintln!("警告: 获取当前工作目录失败: {}，使用当前目录作为开发目录", e);
            PathBuf::from(".")
        }
    }
}

/// 辅助函数
fn current_exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

/// 获取配置目录
pub fn config_dir() -> &'static Path {
    &PATHS.config_dir
}

/// 获取数据/数据库目录
pub fn data_dir() -> &'static Path {
    &PATHS.data_dir
}

/// 获取缓存目录
pub fn cache_dir() -> &'static Path {
    &PATHS.cache_dir
}

/// 获取日志目录
pub fn logs_dir() -> &'static Path {
    &PATHS.logs_dir
}

/// 获取临时文件目录
pub fn temp_dir() -> &'static Path {
    &PATHS.temp_dir
}

/// 获取webview目录
pub fn webview_dir() -> &'static Path {
    &PATHS.webview_dir
}

/// 获取资源文件根目录
pub fn resources_dir() -> &'static Path {
    &PATHS.resources_dir
}

/// 获取当前模式
pub fn current_mode() -> AppMode {
    PATHS.mode
}

/// 判断是否使用自定义目录 (开发模式或便携模式)
///
/// 使用已初始化的 PATHS 而不是重新检测
pub fn is_customized_dir() -> bool {
    matches!(current_mode(), AppMode::Develop | AppMode::Portable)
}

// 单元测试
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_initialization() {
        // 确保所有目录路径都能正常获取
        let _config = config_dir();
        let _data = data_dir();
        let _cache = cache_dir();
        let _logs = logs_dir();
        let _temp = temp_dir();

        // 验证路径不为空
        assert!(!config_dir().as_os_str().is_empty());
        assert!(!data_dir().as_os_str().is_empty());
        assert!(!cache_dir().as_os_str().is_empty());
        assert!(!logs_dir().as_os_str().is_empty());
        assert!(!temp_dir().as_os_str().is_empty());
    }

    #[test]
    fn test_mode_detection() {
        let mode = current_mode();
        // 在 debug 模式下应该是 Develop
        #[cfg(debug_assertions)]
        assert_eq!(mode, AppMode::Develop);

        // 在 release 模式下不是 Develop
        if !cfg!(debug_assertions) {
            assert_ne!(mode, AppMode::Develop);
        }
    }

    #[test]
    fn test_uses_custom_dir_consistency() {
        let mode = current_mode();
        let is_customized = is_customized_dir();

        // 验证 is_customized_dir 和 current_mode 的一致性
        match mode {
            AppMode::Develop | AppMode::Portable => assert!(is_customized),
            AppMode::Install => assert!(!is_customized),
        }
    }

    #[test]
    fn test_current_mode_does_not_repeat_detection() {
        // 验证第一次调用后，后续调用不会重复执行检测逻辑
        let mode1 = current_mode();
        let mode2 = current_mode();
        assert_eq!(mode1, mode2);

        // 验证路径一致
        let data1 = data_dir();
        let data2 = data_dir();
        assert_eq!(data1, data2);
    }
}
