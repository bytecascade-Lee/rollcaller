use crate::config::app_paths;
use crate::util::yaml_flatten_utils;
use anyhow::Context;
use parking_lot::RwLock;
use rbatis::dark_std::err;
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tracing::error;

/// 标记：该配置项必须由用户提供，无默认值
const PROVIDED_BY_USERS: &str = "__PROVIDED_BY_USERS__";

/// 用户配置文件名
const USER_CONFIG_FILENAME: &str = "config.yaml";

/// 需要重启才能生效的键列表
const RESTART_KEYS: &[&str] = &[];

/// 内部状态
struct Inner {
    default_config: HashMap<String, Value>,
    user_config: HashMap<String, Value>,
    temp_config: HashMap<String, Value>,
}

/// 应用配置，线程安全的懒加载单例
pub struct AppConfig {
    inner: RwLock<Inner>,
    user_config_path: PathBuf,
}

/// 懒加载单例
static INSTANCE: LazyLock<AppConfig> = LazyLock::new(|| {
    let default_config = load_default_config();
    let user_config_path = super::app_paths::config_dir().join(USER_CONFIG_FILENAME);
    let user_config = load_user_config(&user_config_path);

    let mut keys: Vec<String> = default_config.keys().cloned().collect();
    keys.sort(); // 按字母顺序排序（区分大小写）
    write_keys_to_file(&keys).expect("构建 config-keys.temp文件失败");

    AppConfig {
        inner: RwLock::new(Inner {
            default_config,
            user_config,
            temp_config: HashMap::new(),
        }),
        user_config_path,
    }
});

/// 从资源目录加载默认配置
fn load_default_config() -> HashMap<String, Value> {
    yaml_flatten_utils::flatten_file_default(&app_paths::resources_dir().join("config/default.yaml"))
        .expect("默认配置扁平化失败")
}

/// 从磁盘加载用户配置，文件不存在或解析失败时返回空 Map
fn load_user_config(path: &Path) -> HashMap<String, Value> {
    if !path.exists() {
        return HashMap::new();
    }
    yaml_flatten_utils::flatten_file_default(path).unwrap_or_else(|e| {
        error!("用户配置扁平化失败: {}", e);
        HashMap::new()
    })
}

/// 删除 user_props 中与 default_props 相同的项
fn filter_user_props(inner: &mut Inner) {
    let default_config = &inner.default_config;
    inner
        .user_config
        .retain(|k, v| default_config.get(k).map_or(true, |def| def != v));
}

/// 将 userProps 写入磁盘
fn flush_to_file(inner: &mut Inner, config_path: &PathBuf) -> anyhow::Result<()> {
    filter_user_props(inner);

    if inner.user_config.is_empty() {
        return Ok(());
    }
    std::fs::write(
        config_path,
        yaml_flatten_utils::unflatten_to_string_default(&inner.user_config)?,
    )?;
    Ok(())
}

#[cfg(debug_assertions)]
fn write_keys_to_file(keys: &[String]) -> anyhow::Result<PathBuf> {
    let file_path = app_paths::temp_dir().join("config-keys.temp");

    // 确保父目录存在
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create temp directory")?;
    }

    let file = File::create(&file_path).context("Failed to create file")?;
    let mut writer = BufWriter::new(file);

    for key in keys {
        writeln!(writer, "{}", key).context("Failed to write key to file")?;
    }

    writer.flush().context("Failed to flush writer")?;
    Ok(file_path)
}

impl AppConfig {
    fn instance() -> &'static Self {
        &INSTANCE
    }

    /// 获取当前生效的配置值
    /// 用户配置 > 默认配置
    pub fn get(key: &str) -> Option<Value> {
        let inner = Self::instance().inner.read();
        // 优先取用户配置
        if let Some(val) = inner.user_config.get(key) {
            return Some(val.clone());
        }
        // 取默认配置，排除 __PROVIDED_BY_USERS__ 哨兵
        inner
            .default_config
            .get(key)
            .filter(|v| v.as_str() != Some(PROVIDED_BY_USERS))
            .cloned()
    }

    /// 修改配置
    /// 写入临时区，不生效，不修改正式配置
    pub fn set(key: &str, value: impl Into<Value>) {
        if key.is_empty() {
            return;
        }
        let config = Self::instance();
        let mut inner = config.inner.write();
        let value = value.into();
        // 直接从 inner 计算当前生效值，避免写锁中再获取读锁
        let current = inner
            .user_config
            .get(key)
            .or_else(|| {
                inner
                    .default_config
                    .get(key)
                    .filter(|v| v.as_str() != Some(PROVIDED_BY_USERS))
            })
            .cloned();
        if Some(value.clone()) == current {
            // 新值与当前生效值相同，移除临时修改
            inner.temp_config.remove(key);
        } else {
            inner.temp_config.insert(key.to_string(), value);
        }
    }

    /// 保存临时区中的所有修改，使其生效并持久化到文件
    pub fn save() -> anyhow::Result<()> {
        let config = Self::instance();
        let mut inner = config.inner.write();
        if inner.temp_config.is_empty() {
            return Ok(());
        }
        // 先 drain 到局部 Vec，避免同时借用 inner 的多个字段
        let temp_entries: Vec<(String, Value)> = inner.temp_config.drain().collect();
        for (key, val) in temp_entries {
            let def_val = inner.default_config.get(&key);
            if def_val.map(|d| d == &val).unwrap_or(false) {
                inner.user_config.remove(&key);
            } else {
                inner.user_config.insert(key, val);
            }
        }
        flush_to_file(&mut inner, &config.user_config_path)?;
        Ok(())
    }

    /// 立刻将制定配置项保存到文件，绕过临时区
    /// 如果对应配置项值未发生变化或过滤后无需写入，则不写入磁盘
    pub fn save_key(key: &str) -> anyhow::Result<()> {
        // ? 随后处理
        Ok(())
    }

    /// 立即将指定配置项及值保存到文件，绕过临时区
    pub fn save_key_value(key: &str, value: impl Into<Value>) -> anyhow::Result<()> {
        if key.is_empty() {
            return Ok(());
        }
        let mut inner = Self::instance().inner.write();
        let v = value.into();
        if inner.default_config.get(key) == Some(&v) {
            inner.user_config.remove(key);
        } else {
            inner.user_config.insert(key.to_string(), v);
        }
        inner.temp_config.remove(key);
        flush_to_file(&mut inner, &Self::instance().user_config_path)?;
        Ok(())
    }

    /// 放弃所有未保存的修改
    pub fn discard() {
        Self::instance().inner.write().temp_config.clear();
    }

    /// 将某项配置恢复为默认值（通过临时区）
    pub fn reset(key: &str) {
        let def_val = Self::instance().inner.read().default_config.get(key).cloned();
        if let Some(val) = def_val {
            Self::set(key, val);
        }
    }

    /// 重新从文件加载配置（会丢失未保存的修改）
    pub fn reload() -> anyhow::Result<()> {
        let config = Self::instance();
        let mut inner = config.inner.write();
        inner.temp_config.clear();
        inner.user_config = load_user_config(&config.user_config_path);
        filter_user_props(&mut inner);
        Ok(())
    }

    /// 未保存的修改数
    pub fn count_unsaved_changes() -> usize {
        Self::instance().inner.read().temp_config.len()
    }

    /// 其中需要重启才能生效的修改数
    pub fn count_restart_changes() -> usize {
        Self::instance()
            .inner
            .read()
            .temp_config
            .keys()
            .filter(|k| RESTART_KEYS.contains(&k.as_str()))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default_config_loaded() {
        let instance = INSTANCE.inner.read();
        assert!(instance.default_config.contains_key("web.local.port"));
        assert!(instance.default_config.contains_key("web.connection.mode"));
        // YAML 中实际的扁平键
        assert!(instance.default_config.contains_key("log.console.level.start"));
    }

    #[test]
    fn test_get_default_value() {
        // port 在 YAML 中是数字类型，用 as_i64 读取
        let port = AppConfig::get("web.local.port");
        assert_eq!(port.as_ref().and_then(|v| v.as_i64()), Some(5173));
    }

    #[test]
    fn test_get_nonexistent_key() {
        assert!(AppConfig::get("nonexistent.key").is_none());
    }

    #[test]
    fn test_set_and_discard() {
        AppConfig::set("test.key", "test_val");
        assert_eq!(AppConfig::count_unsaved_changes(), 1);
        // set 写入 temp，save 前 get 不可见（Java Properties 语义）
        assert!(AppConfig::get("test.key").is_none());

        AppConfig::discard();
        assert_eq!(AppConfig::count_unsaved_changes(), 0);
    }

    #[test]
    fn test_set_same_as_effective_type_matters() {
        // 默认值 web.local.port 是 Number(5173)
        // 传入 String("5173") 类型不同，视为变更（写入 temp）
        AppConfig::set("web.local.port", "5173");
        assert_eq!(
            AppConfig::count_unsaved_changes(),
            1,
            "String(\"5173\") != Number(5173)，应视为变更"
        );
        // 传入 Number(5173) 类型相同，值相同 → 清除 temp
        AppConfig::set("web.local.port", 5173);
        assert_eq!(AppConfig::count_unsaved_changes(), 0);
    }

    #[test]
    fn test_set_numeric_value() {
        AppConfig::set("test.num", 8080);
        assert_eq!(AppConfig::count_unsaved_changes(), 1);
        AppConfig::discard();
    }

    #[test]
    fn test_set_boolean_value() {
        AppConfig::set("test.flag", true);
        assert_eq!(AppConfig::count_unsaved_changes(), 1);
        AppConfig::discard();
    }

    #[test]
    fn test_reset_to_default() {
        AppConfig::set("web.local.port", 9999);
        assert_eq!(AppConfig::count_unsaved_changes(), 1);

        AppConfig::reset("web.local.port");
        // reset → set(key, defaultVal) → default 是 Number(5173)，当前生效值也是 Number(5173)
        // 相同类型相同值 → 移除 temp
        assert_eq!(AppConfig::count_unsaved_changes(), 0);
    }

    #[test]
    fn test_restart_keys_count() {
        AppConfig::set("web.local.port", 8080);
        AppConfig::set("log.console.level.start", "INFO");
        AppConfig::set("some.other.key", "value");
        // RESTART_KEYS 包含 web.local.port 但不包含 log.console.level.start
        assert_eq!(AppConfig::count_restart_changes(), 1);
        AppConfig::discard();
    }

    #[test]
    fn test_empty_key_set_is_noop() {
        AppConfig::set("", "value");
        assert_eq!(AppConfig::count_unsaved_changes(), 0);
    }

    #[test]
    fn test_save_key_bypasses_temp() -> anyhow::Result<()> {
        AppConfig::save_key_value("web.local.port", 9090)?;
        // save_key 直接写入 user_props，get 应立即可见
        assert_eq!(AppConfig::get("web.local.port").as_ref().and_then(|v| v.as_i64()), Some(9090));
        assert_eq!(AppConfig::count_unsaved_changes(), 0);
        // 清理
        AppConfig::save_key_value("web.local.port", 5173)?;
        assert_eq!(AppConfig::get("web.local.port").as_ref().and_then(|v| v.as_i64()), Some(5173));
        Ok(())
    }

    #[test]
    fn test_reload_discards_temp() {
        AppConfig::set("temp.key", "temp_val");
        assert_eq!(AppConfig::count_unsaved_changes(), 1);
        let _ = AppConfig::reload();
        assert_eq!(AppConfig::count_unsaved_changes(), 0);
    }

    #[test]
    fn test_save_and_load_cycle() -> anyhow::Result<()> {
        AppConfig::set("test.save.key", "save_value");
        AppConfig::save()?;

        // save 后 user_props 含有该值，get 可见
        assert_eq!(
            AppConfig::get("test.save.key").as_ref().and_then(|v| v.as_str()),
            Some("save_value")
        );
        assert_eq!(AppConfig::count_unsaved_changes(), 0);

        // 清理
        let config = INSTANCE.inner.write();
        let _ = std::fs::remove_file(&INSTANCE.user_config_path);
        drop(config);
        AppConfig::reload()?;
        assert!(AppConfig::get("test.save.key").is_none());
        Ok(())
    }

    #[test]
    fn test_save_then_get_sees_user_value_not_default() -> anyhow::Result<()> {
        // 覆盖默认值并保存
        AppConfig::save_key_value("web.local.port", 8080)?;
        assert_eq!(AppConfig::get("web.local.port").as_ref().and_then(|v| v.as_i64()), Some(8080));
        // 清理：恢复默认值
        AppConfig::save_key_value("web.local.port", 5173)?;
        Ok(())
    }

    #[test]
    fn test_value_type_preserved_through_cycle() -> anyhow::Result<()> {
        // 数值类型在写回后应保持为数值
        AppConfig::set("test.cycles.num", 42_i64);
        AppConfig::save()?;

        assert_eq!(AppConfig::get("test.cycles.num").as_ref().and_then(|v| v.as_i64()), Some(42));

        let config = INSTANCE.inner.write();
        let _ = std::fs::remove_file(&INSTANCE.user_config_path);
        drop(config);
        AppConfig::reload()?;
        Ok(())
    }
}
