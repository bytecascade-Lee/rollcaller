use crate::config::app_paths;
use parking_lot::RwLock;
use rbatis::RBatis;
use rbdc_sqlite::SqliteDriver;
use std::sync::{Arc, LazyLock};

/// 数据库连接池单例
static DATABASE_POOL: LazyLock<DatabasePool> = LazyLock::new(|| DatabasePool {
    pool: RwLock::new(None),
});

/// 数据库连接池管理器
///
/// 管理 SQLite 数据库连接池的生命周期，采用懒加载初始化策略。
/// 内部使用 `Arc` 包装连接池实例，确保克隆时只增加引用计数而不复制数据。
struct DatabasePool {
    pool: RwLock<Option<Arc<RBatis>>>,
}

impl DatabasePool {
    /// 获取单例实例
    fn instance() -> &'static Self {
        &DATABASE_POOL
    }

    /// 初始化数据库连接池
    ///
    /// 如果连接池已存在则直接返回，否则创建新连接池并存储。
    /// 该方法是内部方法，由 `database()` 在需要时自动调用。
    async fn init(&self) -> anyhow::Result<()> {
        // 快速路径：只读检查（不跨 await，安全释放）
        if self.pool.read().is_some() {
            return Ok(());
        }

        let rb = Self::create_pool().await?;

        // 慢路径：加写锁，双重检查
        let mut guard = self.pool.write();
        if guard.is_some() {
            return Ok(());
        }
        *guard = Some(Arc::new(rb));
        Ok(())
    }

    /// 创建连接池
    ///
    /// 使用 `app_paths::data_dir()` 获取数据库文件路径，
    /// 连接 SQLite 数据库并返回 `RBatis` 实例。
    /// 如果数据库文件不存在，SQLite 会自动创建。
    async fn create_pool() -> anyhow::Result<RBatis> {
        let rb = RBatis::new();
        let db_path = app_paths::data_dir()
            .join("sqlite.db")
            .to_str()
            .expect("数据库路径转换失败")
            .to_string();

        rb.link(SqliteDriver {}, format!("sqlite:{}", db_path).as_str())
            .await?;

        Ok(rb)
    }

    /// 获取连接池（内部使用）
    ///
    /// 返回 `Option<Arc<RBatis>>`，如果连接池已初始化则包含 `Arc` 引用。
    /// 克隆 `Arc` 只增加引用计数，不复制底层数据。
    fn get_pool(&self) -> Option<Arc<RBatis>> {
        self.pool.read().clone()
    }
}

/// 获取数据库连接池
///
/// 返回 `Arc<RBatis>` 用于执行数据库操作。
/// 连接池采用懒加载策略，首次调用时自动初始化。
/// 多次调用返回同一个连接池实例（通过 `Arc` 共享）。
///
/// # 返回
/// - `Ok(Arc<RBatis>)`: 成功获取连接池
/// - `Err(std::error::Error)`: 初始化或连接失败
///
/// # 示例
/// ```no_run
/// use crate::config::database_pool;
///
/// async fn example() -> anyhow::Result<()> {
///     let db = database_pool::database().await?;
///     // 使用 db 执行数据库操作
///     Ok(())
/// }
/// ```
pub async fn database() -> anyhow::Result<Arc<RBatis>> {
    let instance = DatabasePool::instance();

    // 先尝试读锁快速路径
    {
        let guard = instance.pool.read();
        if let Some(ref pool) = *guard {
            return Ok(pool.clone());
        }
    }

    // 未初始化则获取写锁进行初始化
    instance.init().await?;

    // 初始化后再次获取读锁返回
    let guard = instance.pool.read();
    // 这里 unwrap 是安全的，因为刚刚初始化完成
    Ok(guard.as_ref().unwrap().clone())
}

/// 关闭数据库连接池
///
/// 显式关闭连接池，释放所有数据库连接。
/// 通常用于应用优雅退出时清理资源。
/// 关闭后可以再次调用 `database()` 重新初始化。
///
/// # 返回
/// - `Ok(())`: 成功关闭
/// - `Err(std::error::Error)`: 关闭失败
///
/// # 示例
/// ```no_run
/// use crate::config::database_pool;
///
/// async fn shutdown() -> anyhow::Result<()> {
///     database_pool::close().await?;
///     Ok(())
/// }
/// ```
pub async fn close() -> anyhow::Result<()> {
    let instance = DatabasePool::instance();
    let mut guard = instance.pool.write();
    if let Some(pool) = guard.take() {
        // RBatis 没有显式的 close 方法，drop 时会自动清理
        drop(pool);
    }
    Ok(())
}

/// 重置数据库连接池
///
/// 关闭现有连接池并立即重新初始化。
/// 主要用于测试场景，或在连接池状态异常时强制重建。
///
/// # 注意
/// - 重置过程中，所有正在使用的连接（通过 `Arc` 持有）不会立即失效，
///   但旧连接池将在所有持有者释放后自动清理。
/// - 新初始化将创建全新的连接池实例。
///
/// # 返回
/// - `Ok(())`: 重置成功
/// - `Err(std::error::Error)`: 关闭或重新初始化失败
///
/// # 示例
/// ```no_run
/// use crate::config::database_pool;
///
/// async fn force_reconnect() -> anyhow::Result<()> {
///     database_pool::reset().await?;
///     Ok(())
/// }
/// ```
#[allow(dead_code)]
pub async fn reset() -> anyhow::Result<()> {
    close().await?;
    database().await?;
    Ok(())
}
