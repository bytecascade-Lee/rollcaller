//! 全局命名 HTTP 客户端(进程级 state)。
//!
//! 所有 client 以 key 存放在注册表中，对外返回 `&'static Client`(与进程同生命周期，连接池可复用)。
//! 内置两个命名实例:
//!
//! - `default` — 常规请求(整体超时 30s 等)，经 [`client()`] 获取;
//! - `download` — 大文件下载专用(不设整体超时，仅连接/读空闲超时,避免整体超时截断大包)，经 [`download()`] 获取。
//!
//! 需要不同配置的模块可自行 [`register`] 一个命名 client，之后用 [`get`] 按 key 取用;
//! 内置 key 常量见 [`DEFAULT_CLIENT_KEY`] 与 [`DOWNLOAD_CLIENT_KEY`]。

use reqwest::Client;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

/// 内置常规 client 的注册 key
pub const DEFAULT_CLIENT_KEY: &str = "default";

/// 内置下载 client 的注册 key
pub const DOWNLOAD_CLIENT_KEY: &str = "download";

/// reqwest client 的可配置项;字段全部公开,便于在默认值基础上覆盖后注册新实例
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// 整体请求超时;`None` 表示不设整体超时(仅靠 `read_timeout` 兜底,适合大文件下载)
    pub timeout: Option<Duration>,
    /// 两次读取之间的空闲超时
    pub read_timeout: Option<Duration>,
    /// 建立连接的超时
    pub connect_timeout: Duration,
    /// 每 host 空闲连接池上限
    pub pool_max_idle_per_host: usize,
    /// 空闲连接回收超时
    pub pool_idle_timeout: Duration,
    /// 默认 UA(单个请求仍可在请求层用 header 覆盖)
    pub user_agent: String,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(30)),
            read_timeout: None,
            connect_timeout: Duration::from_secs(10),
            pool_max_idle_per_host: 10,
            pool_idle_timeout: Duration::from_secs(90),
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36 Edg/151.0.0.0".to_string(),
        }
    }
}

impl HttpClientConfig {
    /// 下载专用配置:不设整体超时(大包不被截断)，仅连接超时 + 读空闲超时兜底
    pub fn for_download() -> Self {
        Self {
            timeout: None,
            read_timeout: Some(Duration::from_secs(60)),
            ..Self::default()
        }
    }
}

/// 命名注册表:key → 全局存活的 client(构建后以 `Box::leak` 提升为 `&'static`,与进程同生命周期)
static REGISTRY: LazyLock<Mutex<HashMap<String, &'static Client>>> = LazyLock::new(|| {
    let mut clients = HashMap::new();
    clients.insert(DEFAULT_CLIENT_KEY.to_string(), build(&HttpClientConfig::default()));
    clients.insert(DOWNLOAD_CLIENT_KEY.to_string(), build(&HttpClientConfig::for_download()));
    Mutex::new(clients)
});

/// 按配置构建 client，并以 `&'static` 形式交给注册表持有(构建不发起网络 IO)
fn build(config: &HttpClientConfig) -> &'static Client {
    let mut builder = Client::builder()
        .connect_timeout(config.connect_timeout)
        .pool_max_idle_per_host(config.pool_max_idle_per_host)
        .pool_idle_timeout(config.pool_idle_timeout)
        .user_agent(config.user_agent.as_str());
    if let Some(t) = config.timeout {
        builder = builder.timeout(t);
    }
    if let Some(t) = config.read_timeout {
        builder = builder.read_timeout(t);
    }
    Box::leak(Box::new(builder.build().expect("构建 reqwest client 失败")))
}

/// 按自定义配置注册一个命名 client；key 已存在时返回错误(不覆盖)
///
/// 可以通过 [`is_registered`] 查询是否已注册
/// 注册成功后返回该 client 的 `&'static` 引用,之后也可用 [`get`] 按 key 取用。
pub fn register(key: impl Into<String>, config: HttpClientConfig) -> Result<&'static Client, String> {
    let key = key.into();
    let mut clients = REGISTRY.lock().expect("HTTP client 注册表锁中毒");
    if clients.contains_key(&key) {
        return Err(format!("HTTP client [{key}] 已注册,请改用其它 key"));
    }
    let client = build(&config);
    clients.insert(key, client);
    Ok(client)
}

/// 按 key 查询某个 client 是否已注册，未注册时返回 `false`
pub fn is_registered(key: &str) -> bool {
    REGISTRY.lock().expect("HTTP client 注册表锁中毒").get(key).is_some()
}

/// 按 key 获取已注册的 client；未注册时返回 `None`
pub fn get(key: &str) -> Option<&'static Client> {
    REGISTRY.lock().expect("HTTP client 注册表锁中毒").get(key).copied()
}

/// 获取内置常规 client (整体超时 30s)，用于普通 API 请求
pub fn client() -> &'static Client {
    get(DEFAULT_CLIENT_KEY).expect("内置 default client 一定存在")
}

/// 获取内置下载 client (不设整体超时，仅连接/读空闲超时)，用于大文件下载
pub fn download() -> &'static Client {
    get(DOWNLOAD_CLIENT_KEY).expect("内置 download client 一定存在")
}
