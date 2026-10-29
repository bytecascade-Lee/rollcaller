use crate::config::app_paths;
use crate::database::database_pool::{database, db_path};
use crate::database::migration;
use anyhow::{Context, Result};

pub async fn init() -> Result<()> {
    let rb = database()
        .await
        .with_context(|| format!("初始化数据库连接池失败, 数据目录: {}", app_paths::data_dir().display()))?;
    migration::run(&rb, &db_path()).await.context("数据库迁移失败")
}
