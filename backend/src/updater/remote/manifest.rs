use crate::updater::model::UpdateManifest;
use anyhow::{anyhow, Context};

/// 解析目标版本清单 JSON，返回结构体与原始 JSON
pub fn parse_manifest(text: &str) -> anyhow::Result<UpdateManifest> {
    serde_json::from_str(text).context(anyhow!("清单格式不符合规范"))
}
