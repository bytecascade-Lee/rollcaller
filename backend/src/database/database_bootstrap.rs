use crate::config::app_paths;
use crate::database::database_pool::database;
use crate::service::database_migrate_service::migrate;
use anyhow::{Context, Result};

pub async fn init() -> Result<()> {
    let migration_dir = app_paths::resources_dir().join("database/migrations");
    let rb = database()
        .await
        .with_context(|| {
            format!(
                "初始化数据库连接池失败, 数据目录: {}",
                app_paths::data_dir().display()
            )
        })?;
    migrate(&*rb, &migration_dir)
        .await
        .with_context(|| format!("数据库迁移失败, 迁移目录: {}", migration_dir.display()))
}
