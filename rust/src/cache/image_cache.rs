//! 图片两级缓存：内存/DB 存磁盘路径，磁盘存图片；对外返回路径字符串。
//!
//! 自带持久化（`image_db` 表），无失效语义；启动时清理过期图片（DB 记录 + 磁盘文件）。

use anyhow::Context;
use moka::future::Cache;
use sea_orm::entity::prelude::*;
use sea_orm::Set;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// image_db 实体：key + 磁盘路径 + 写入时间
pub mod image_db {
    use sea_orm::entity::prelude::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "image_cache")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub key: String,
        pub path: String,
        pub created_at: i64,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// 图片两级缓存
pub struct ImageCache {
    /// 内存缓存（key -> 磁盘路径字符串，无 TTL，容量 LRU）
    mem: Cache<String, String>,
    /// 自己的持久化连接（image_cache 表）
    db: DatabaseConnection,
    /// 图片磁盘根目录
    disk_dir: PathBuf,
}

impl ImageCache {
    /// 构建缓存：打开 DB 连接并自动建表（磁盘目录为 `cache_dir/images`）
    pub async fn new(cache_dir: &Path) -> anyhow::Result<Self> {
        let disk_dir = cache_dir.join("images");
        tokio::fs::create_dir_all(&disk_dir)
            .await
            .with_context(|| format!("创建图片缓存目录失败: {}", disk_dir.display()))?;
        let db = crate::cache::connect_db(cache_dir).await?;
        // 自动建表（Schema Registry，幂等）
        db.get_schema_registry("rust_lib_komikku::*").sync(&db).await?;
        let cache = Self {
            mem: moka::future::Cache::builder().max_capacity(100_000).build(),
            db,
            disk_dir,
        };
        Ok(cache)
    }

    /// 获取图片磁盘路径字符串：内存/DB 命中（文件存在）→ 返回路径，否则 `fetch` 下载并落盘
    pub async fn get<F, Fut>(&self, key: &str, fetch: F) -> anyhow::Result<String>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = anyhow::Result<Vec<u8>>>,
    {
        let db_key = key.to_string();

        // 1. 内存命中
        if let Some(path) = self.mem.get(&db_key).await {
            return Ok(path);
        }

        // 2. DB 命中（文件存在）→ 回填内存
        if let Some(path) = self.get_from_db(&db_key).await? {
            self.mem.insert(db_key.clone(), path.clone()).await;
            return Ok(path);
        }

        // 3. 下载 → 落盘 → 写 DB + 内存（文件名 = md5(key)，不带扩展名）
        let bytes = fetch().await?;
        let pb = self.disk_dir.join(format!("{:x}", md5::compute(db_key.as_bytes())));
        tokio::fs::write(&pb, &bytes)
            .await
            .with_context(|| format!("写入图片缓存失败: {}", pb.display()))?;
        let path = pb.to_string_lossy().into_owned();
        self.put_to_db(&db_key, &path).await?;
        self.mem.insert(db_key, path.clone()).await;
        Ok(path)
    }

    /// 从 DB 读取路径并校验磁盘文件：无记录或文件丢失返回 `None`（文件丢失时删掉脏记录）
    async fn get_from_db(&self, key: &str) -> anyhow::Result<Option<String>> {
        let Some(row) = image_db::Entity::find_by_id(key.to_string()).one(&self.db).await? else {
            return Ok(None);
        };

        let pb = PathBuf::from(&row.path);
        // 用 tokio 异步检查文件是否存在（避免阻塞 runtime）
        if tokio::fs::metadata(&pb).await.map(|m| m.is_file()).unwrap_or(false) {
            return Ok(Some(row.path));
        }

        // 磁盘文件丢失：删脏记录，走重新下载
        tracing::warn!("图片缓存文件丢失，重新下载: {key}");
        self.delete_from_db(key).await?;
        Ok(None)
    }

    /// 写入 DB（UPSERT：key 已存在则覆盖 path 与 created_at）
    async fn put_to_db(&self, key: &str, path: &str) -> anyhow::Result<()> {
        image_db::Entity::insert(image_db::ActiveModel {
            key: Set(key.to_string()),
            path: Set(path.to_string()),
            created_at: Set(crate::cache::now_ts()),
        })
        .on_conflict(
            sea_orm::sea_query::OnConflict::new()
                .update_column(image_db::COLUMN.path)
                .update_column(image_db::COLUMN.created_at)
                .to_owned(),
        )
        .exec(&self.db)
        .await?;
        Ok(())
    }

    /// 删除 DB 记录
    async fn delete_from_db(&self, key: &str) -> anyhow::Result<()> {
        image_db::Entity::delete_by_id(key.to_string()).exec(&self.db).await?;
        Ok(())
    }

    /// 清理：删除 created_at 早于 max_age 的图片（DB 行 + 磁盘文件）
    pub async fn cleanup_expired(&self, max_age: Duration) -> anyhow::Result<()> {
        let cutoff = crate::cache::now_ts() - max_age.as_secs() as i64;
        let rows = image_db::Entity::find()
            .filter(image_db::COLUMN.created_at.lt(cutoff))
            .all(&self.db)
            .await?;
        let paths: Vec<String> = rows.iter().map(|m| m.path.clone()).collect();

        image_db::Entity::delete_many()
            .filter(image_db::COLUMN.created_at.lt(cutoff))
            .exec(&self.db)
            .await?;

        for p in &paths {
            match tokio::fs::remove_file(p).await {
                Ok(_) => tracing::info!("清理过期图片: {p}"),
                Err(e) => tracing::warn!("清理过期图片文件失败 {p}: {e}"),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("koma_image_cache_test_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        base
    }

    #[tokio::test]
    async fn test_fetch_once_and_disk() {
        let dir = temp_dir("fetch");
        let cache = ImageCache::new(&dir).await.unwrap();
        assert!(dir.join("cache.db").is_file(), "cache.db 应已创建");

        let calls = Arc::new(AtomicUsize::new(0));
        // 模拟图片字节，文件名固定为 md5(key)
        let bytes: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0, 0, 0, 0, 0];
        let fetch = || {
            calls.fetch_add(1, Ordering::SeqCst);
            let b = bytes.clone();
            async move { Ok::<_, anyhow::Error>(b) }
        };

        let p1 = cache.get("photo:1:00001", fetch).await.unwrap();
        let p2 = cache.get("photo:1:00001", fetch).await.unwrap();
        assert_eq!(p1, p2);
        assert_eq!(calls.load(Ordering::SeqCst), 1, "第二次应命中缓存");
        assert!(Path::new(&p1).is_file(), "磁盘文件应存在");
    }
}
