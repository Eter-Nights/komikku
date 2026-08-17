//! 全局运行时状态：数据目录 + 应用配置 + 缓存 + 业务服务，跨 FFI 调用共享。

use crate::cache::{ApiCache, ImageCache};
use crate::comic_source::jmcomic::client::{CategorySort, FavoriteSort, SearchSort};
use crate::config::Config;
use crate::service::jm::JmService;
use crate::service::model::{
    AlbumDetailInfo, CategoryInfo, ChapterInfo, FavoriteInfo, PromoteListInfo, PromoteSectionInfo, SearchInfo,
    SerializationInfo, ToggleType, UserInfo,
};
use anyhow::Context;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::RwLock;

static STATE: OnceLock<AppState> = OnceLock::new();

/// 应用运行时状态
pub struct AppState {
    /// 数据目录路径
    pub path: String,
    /// 应用配置（读多写少，用 RwLock）
    pub config: RwLock<Config>,
    /// JM 业务服务（格式转换 + 业务逻辑 + 缓存，缓存实例在 init 时创建并注入）
    service: JmService,
}

impl AppState {
    /// 返回 `Ok(true)` 表示完成初始化；`Ok(false)` 表示已初始化过（防止多次调用）；`Err` 表示初始化失败。
    pub async fn init(path: String) -> anyhow::Result<bool> {
        if STATE.get().is_some() {
            return Ok(false);
        }

        // 1. 初始化日志：只保留本 crate（backend）的日志，过滤掉其它库的日志
        let lib_module_path = module_path!();
        let lib_target = lib_module_path
            .split("::")
            .next()
            .context(format!("解析lib_target失败: lib_module_path={lib_module_path}"))?;
        tracing_subscriber::fmt()
            .with_env_filter(format!("{lib_target}=trace"))
            .with_target(false)
            .with_file(true)
            .with_line_number(true)
            .with_timer(tracing_subscriber::fmt::time::LocalTime::rfc_3339())
            .init();

        // 2. 创建数据目录
        let dir = Path::new(&path);
        tokio::fs::create_dir_all(dir).await?;

        // 3. 初始化配置
        let config = Config::load_from_dir(dir).await?;

        // 4. 构建共享缓存（异步构建，之后在异步方法里使用）
        let api_cache = Arc::new(ApiCache::new(dir).await?);
        let image_cache = Arc::new(ImageCache::new(dir).await?);

        // 5. 启动清理：删除缓存时间早于保留天数的过期数据（DB 记录 + 磁盘文件）
        let max_age = Duration::from_secs(config.cache_cleanup_days * 24 * 3600);
        api_cache.cleanup_expired(max_age).await?;
        image_cache.cleanup_expired(max_age).await?;

        // 6. 构建业务服务（注入共享缓存，缓存逻辑在服务内部）
        let service = JmService::new(&config, api_cache.clone(), image_cache.clone());

        let ok = STATE
            .set(AppState {
                path,
                config: RwLock::new(config),
                service,
            })
            .is_ok();
        Ok(ok)
    }

    /// 获取全局状态引用（未初始化时返回错误）
    pub fn get_app() -> anyhow::Result<&'static AppState> {
        STATE
            .get()
            .ok_or_else(|| anyhow::anyhow!("后端未初始化，请先调用 init_rust"))
    }

    /// 获取当前配置快照
    pub async fn get_config() -> anyhow::Result<Config> {
        Ok(Self::get_app()?.config.read().await.clone())
    }

    /// 更新配置：写入内存并持久化，域名/代理/并发数变化时重建客户端（保留登录态）
    pub async fn update_config(new_config: Config) -> anyhow::Result<()> {
        let app = Self::get_app()?;
        // 1. 先写入内存，再直接持久化内存中的最新值
        let mut cfg = app.config.write().await;
        let changed = cfg.needs_reload(&new_config);
        *cfg = new_config;
        // 2. 需要重建客户端时通知业务服务（保留 cookie jar，登录态不丢）
        if changed {
            app.service.reload(&cfg);
        }
        cfg.save_to_dir(Path::new(&app.path)).await?;
        Ok(())
    }

    // ---------- 透传 JmService 接口（缓存逻辑已下沉到各业务服务内部） ----------

    /// 登录，返回用户信息（不缓存）
    pub async fn login(&self, username: &str, password: &str) -> anyhow::Result<UserInfo> {
        self.service.login(username, password).await
    }

    /// 收藏列表（不缓存）
    pub async fn get_favorite(&self, folder_id: i64, page: i32, sort: FavoriteSort) -> anyhow::Result<FavoriteInfo> {
        self.service.get_favorite(folder_id, page, sort).await
    }

    /// 切换收藏（不缓存）
    pub async fn toggle_favorite(&self, album_id: i64) -> anyhow::Result<ToggleType> {
        self.service.toggle_favorite(album_id).await
    }

    /// 专辑详情（服务内部 API 缓存）
    pub async fn get_album(&self, id: i64) -> anyhow::Result<AlbumDetailInfo> {
        self.service.get_album(id).await
    }

    /// 章节：id、图片名列表、scramble_id（服务内部 API 缓存）
    pub async fn get_chapter(&self, chapter_id: i64) -> anyhow::Result<ChapterInfo> {
        self.service.get_chapter(chapter_id).await
    }

    /// 首页推荐分组（服务内部 API 缓存）
    pub async fn get_promote(&self) -> anyhow::Result<Vec<PromoteSectionInfo>> {
        self.service.get_promote().await
    }

    /// 推荐分组下的分页专辑列表（服务内部 API 缓存）
    pub async fn get_promote_list(&self, id: i64, page: i32) -> anyhow::Result<PromoteListInfo> {
        self.service.get_promote_list(id, page).await
    }

    /// 每周连载更新：type 为 all/manga/hanman，date 为 0~7（服务内部 API 缓存）
    pub async fn get_serialization(
        &self,
        serial_type: &str,
        date: &str,
        page: i32,
    ) -> anyhow::Result<SerializationInfo> {
        self.service.get_serialization(serial_type, date, page).await
    }

    /// 搜索（服务内部 API 缓存）
    pub async fn search(&self, keyword: &str, page: i32, sort: SearchSort) -> anyhow::Result<SearchInfo> {
        self.service.search(keyword, page, sort).await
    }

    /// 分类列表（服务内部 API 缓存）
    pub async fn get_categories(&self) -> anyhow::Result<CategoryInfo> {
        self.service.get_categories().await
    }

    /// 分类下的专辑列表（服务内部 API 缓存，结构与搜索一致）
    pub async fn get_categories_filter(
        &self,
        category: &str,
        page: i32,
        sort: CategorySort,
    ) -> anyhow::Result<SearchInfo> {
        self.service.get_categories_filter(category, page, sort).await
    }

    /// 封面，返回磁盘路径字符串（服务内部图片磁盘缓存）
    pub async fn get_cover(&self, image_name: &str) -> anyhow::Result<String> {
        self.service.get_cover(image_name).await
    }

    /// 章节图片，返回磁盘路径字符串（服务内部图片磁盘缓存，已按 scramble_id 还原分块）
    pub async fn get_photo(&self, scramble_id: i32, chapter_id: i32, image_name: &str) -> anyhow::Result<String> {
        self.service.get_photo(scramble_id, chapter_id, image_name).await
    }
}
