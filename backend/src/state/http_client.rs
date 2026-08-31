use reqwest::Client;
use std::sync::LazyLock;
use std::time::Duration;


#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub pool_max_idle_per_host: usize,
    pub pool_idle_timeout: Duration,
    pub user_agent: String,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            pool_max_idle_per_host: 10,
            pool_idle_timeout: Duration::from_secs(90),
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36 Edg/151.0.0.0".to_string(),
        }
    }
}

struct HttpClientManager {
    client: Client,
    config: HttpClientConfig,
}

impl HttpClientManager {
    fn new(config: HttpClientConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .pool_max_idle_per_host(config.pool_max_idle_per_host)
            .pool_idle_timeout(config.pool_idle_timeout)
            .user_agent(&config.user_agent)
            .build()
            .expect("Failed to build reqwest client");

        Self { client, config }
    }

    fn get_client(&self) -> &Client {
        &self.client
    }

    fn get_config(&self) -> &HttpClientConfig {
        &self.config
    }
}

static HTTP_CLIENT: LazyLock<HttpClientManager> = LazyLock::new(|| {
    let config = HttpClientConfig::default();
    HttpClientManager::new(config)
});


/// 获取全局 HTTP Client 的引用
pub fn get_client() -> &'static Client {
    HTTP_CLIENT.get_client()
}

/// 获取当前 Client 的配置信息
pub fn get_config() -> &'static HttpClientConfig {
    HTTP_CLIENT.get_config()
}
