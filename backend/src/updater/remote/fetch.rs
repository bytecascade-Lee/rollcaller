use anyhow::{anyhow, Context};
use reqwest::Client;

/// 拉取远程字节
///
/// 返回原始字节而非 `String`：清单要按字节计算 sha256 / 验签，转成 `String` 再回编码会
/// 引入一次不可见的规范化（BOM、非法 UTF-8 替换），摘要可能会对不上。
pub async fn fetch_bytes(client: &Client, url: &str) -> anyhow::Result<Vec<u8>> {
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
    Ok(response
        .bytes()
        .await
        .context(anyhow!("检查更新失败：读取响应失败"))?
        .to_vec())
}

/// 拉取远程文本
pub async fn fetch_json_text(client: &Client, url: &str) -> anyhow::Result<String> {
    let bytes = fetch_bytes(client, url).await?;
    String::from_utf8(bytes).context(anyhow!("检查更新失败：{url}响应不是 UTF-8 文本"))
}
