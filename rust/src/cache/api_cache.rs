//! API 两级缓存：内存(moka) → DB(sea_orm) → 回源；TTL 1h 过期后重新请求。
//!
//! key/value 固定为 String，自带持久化（`api_db` 表），`fetch` 直接返回 `Result<String>`。

use moka::future::Cache;
use sea_orm::entity::prelude::*;
use sea_orm::Set;
use std::future::Future;
use std::path::Path;
use std::time::Duration;

/// API 缓存 TTL（数据新鲜度），固定 1 小时
const API_CACHE_TTL: Duration = Duration::from_secs(3600);

/// api_db 实体：key + 序列化后的 JSON + 写入时间
pub mod api_db {
    use sea_orm::entity::prelude::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "api_cache")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub key: String,
        pub value: String,
        pub created_at: i64,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// API 两级缓存（key / value 均固定为 String）
pub struct ApiCache {
    /// 内存缓存（TTL 过期）
    mem: Cache<String, String>,
    /// 自己的持久化连接（api_cache 表）
    db: DatabaseConnection,
}

impl ApiCache {
    /// 构建缓存：打开自己的 DB 连接并自动建表（DB 物理清理由 `cleanup_expired` 触发）
    pub async fn new(cache_dir: &Path) -> anyhow::Result<Self> {
        let db = crate::cache::connect_db(cache_dir).await?;
        // 自动建表（Schema Registry，幂等）
        db.get_schema_registry("rust_lib_komikku::*").sync(&db).await?;
        let cache = Self {
            mem: moka::future::Cache::builder()
                .time_to_live(API_CACHE_TTL)
                .max_capacity(100_000)
                .build(),
            db,
        };
        Ok(cache)
    }

    /// 取缓存值：内存 → DB（未过期，回填内存）→ `fetch` 回源并写回两级缓存
    pub async fn get<F, Fut>(&self, key: &str, fetch: F) -> anyhow::Result<String>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = anyhow::Result<String>>,
    {
        // 1. 内存命中
        if let Some(v) = self.mem.get(key).await {
            return Ok(v);
        }

        // 2. DB 命中（未过期）→ 回填内存
        if let Some(v) = self.get_from_db(key).await? {
            self.mem.insert(key.to_string(), v.clone()).await;
            return Ok(v);
        }

        // 3. 回源，写回两级缓存
        let v = fetch().await?;
        self.put_to_db(key, &v).await?;
        self.mem.insert(key.to_string(), v.clone()).await;
        Ok(v)
    }

    /// 从 DB 读取：未命中或已过期返回 `None`
    async fn get_from_db(&self, key: &str) -> anyhow::Result<Option<String>> {
        let Some(row) = api_db::Entity::find_by_id(key.to_string()).one(&self.db).await? else {
            return Ok(None);
        };

        // 已过期
        let now = crate::cache::now_ts();
        if now - row.created_at >= API_CACHE_TTL.as_secs() as i64 {
            return Ok(None);
        }

        Ok(Some(row.value))
    }

    /// 写入 DB（UPSERT：key 已存在则覆盖 value 与 created_at）
    async fn put_to_db(&self, key: &str, value: &str) -> anyhow::Result<()> {
        api_db::Entity::insert(api_db::ActiveModel {
            key: Set(key.to_string()),
            value: Set(value.to_string()),
            created_at: Set(crate::cache::now_ts()),
        })
        .on_conflict(
            sea_orm::sea_query::OnConflict::new()
                .update_column(api_db::COLUMN.value)
                .update_column(api_db::COLUMN.created_at)
                .to_owned(),
        )
        .exec(&self.db)
        .await?;
        Ok(())
    }

    /// 清理过期缓存：删除 DB 中 `created_at` 早于 `max_age` 的行，返回删除行数
    /// （内存 moka 自带 TTL 自动过期，这里只做 DB 物理清理）
    pub async fn cleanup_expired(&self, max_age: Duration) -> anyhow::Result<usize> {
        let cutoff = crate::cache::now_ts() - max_age.as_secs() as i64;
        let result = api_db::Entity::delete_many()
            .filter(api_db::COLUMN.created_at.lt(cutoff))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("koma_api_cache_test_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        base
    }

    #[tokio::test]
    async fn test_fetch_once_and_db_recovery() {
        let dir = temp_dir("fetch");
        let cache = ApiCache::new(&dir).await.unwrap();
        assert!(dir.join("cache.db").is_file(), "cache.db 应已创建");

        let calls = Arc::new(AtomicUsize::new(0));
        let fetch = || {
            calls.fetch_add(1, Ordering::SeqCst);
            async move { Ok::<_, anyhow::Error>("hello".to_string()) }
        };

        assert_eq!(cache.get("k", fetch).await.unwrap(), "hello");
        assert_eq!(cache.get("k", fetch).await.unwrap(), "hello");
        assert_eq!(calls.load(Ordering::SeqCst), 1, "第二次应命中内存缓存");

        // 新实例（内存清空）应命中 DB
        let cache2 = ApiCache::new(&dir).await.unwrap();
        let calls2 = Arc::new(AtomicUsize::new(0));
        let fetch2 = || {
            calls2.fetch_add(1, Ordering::SeqCst);
            async move { Ok::<_, anyhow::Error>("other".to_string()) }
        };
        assert_eq!(cache2.get("k", fetch2).await.unwrap(), "hello", "应从 DB 恢复");
        assert_eq!(calls2.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let dir = temp_dir("cleanup");
        let cache = ApiCache::new(&dir).await.unwrap();

        // 一条新记录（保留）
        cache.put_to_db("fresh", "v").await.unwrap();
        // 手工插入一条已过期记录（created_at 远早于 now - retention(7 天)）
        api_db::Entity::insert(api_db::ActiveModel {
            key: Set("stale".to_string()),
            value: Set("old".to_string()),
            created_at: Set(crate::cache::now_ts() - 10_000_000),
        })
        .exec(&cache.db)
        .await
        .unwrap();

        let deleted = cache.cleanup_expired(Duration::from_secs(7 * 24 * 3600)).await.unwrap();
        assert_eq!(deleted, 1, "应删除 1 条过期记录");
        assert!(cache.get_from_db("stale").await.unwrap().is_none());
        assert!(cache.get_from_db("fresh").await.unwrap().is_some());
    }
}
