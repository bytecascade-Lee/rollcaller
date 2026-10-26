/// # 即将被废弃

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

    #[cfg(debug_assertions)]
    {
        let mut keys: Vec<String> = default_config.keys().cloned().collect();
        keys.sort(); // 按字母顺序排序（区分大小写）
        write_keys_to_file(&keys).expect("构建 config.key文件失败");
    }

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
    let file_path = app_paths::resources_dir().join("develop/config.key");

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


