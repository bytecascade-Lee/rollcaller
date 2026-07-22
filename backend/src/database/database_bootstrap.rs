use crate::config::app_paths;
use crate::database::database_pool::database;
use crate::service::database_migrate_service::migrate;
use anyhow::Result;

pub async fn init() -> Result<()> {
    migrate(
        &*database().await?,
        app_paths::resources_dir()
            .join("database/migrations")
            .as_path(),
    )
    .await
}
