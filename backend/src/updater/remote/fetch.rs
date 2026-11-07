use anyhow::{anyhow, Context};
use reqwest::Client;

/// 拉取远程文本
pub async fn fetch_json_text(client: &Client, url: &str) -> anyhow::Result<String> {
    let response = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .context(anyhow!("检查更新失败：网络错误"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("检查更新失败：{url}服务器返回 HTTP {status}"));
    }
    response.text().await.context(anyhow!("检查更新失败：读取响应失败"))
}
