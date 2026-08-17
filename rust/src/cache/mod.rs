//! 通用两级缓存：内存(moka) + 持久化(sea_orm/SQLite)，供各漫画源复用。
//!
//! API 缓存：内存 → DB，TTL 1h 过期回源；图片缓存：内存/DB 存路径，磁盘存图片。

pub mod api_cache;
pub mod image_cache;

pub use api_cache::ApiCache;
pub use image_cache::ImageCache;

use sea_orm::{DatabaseConnection, SqlxSqliteConnector};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 当前 Unix 时间戳（秒）
pub fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 打开（或创建）`cache_dir/cache.db`，返回连接。
///
/// 直接用 sqlx 的 `SqlitePool` 再包成 `DatabaseConnection`（sea-orm 的 `Database::connect` 无法解析 Windows 绝对路径）
pub(crate) async fn connect_db(cache_dir: &Path) -> anyhow::Result<DatabaseConnection> {
    tokio::fs::create_dir_all(cache_dir).await?;
    let db_path = cache_dir.join("cache.db");

    let opts = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));
    // 单写者池，规避 SQLite 并发写锁
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await?;

    Ok(SqlxSqliteConnector::from_sqlx_sqlite_pool(pool))
}
