use crate::cmd::ipc_refuse;
use crate::logger;
use crate::logger::policy::LogLevel;

/// 运行期切换文件层日志级别
///
/// 入参暂时设置为字符串，后期可考虑枚举
#[tauri::command]
pub fn logger_set_level(level: String) -> Result<String, String> {
    let Some(level) = LogLevel::parse(&level) else {
        return Err(ipc_refuse("logger_set_level", format!("未知的日志级别：{level}")));
    };
    logger::set_level(level).map_err(|e| ipc_refuse("logger_set_level", e))?;
    Ok(level.to_string())
}
