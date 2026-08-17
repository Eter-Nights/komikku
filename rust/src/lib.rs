mod cache;
mod comic_source;
mod config;
mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */
mod service;
mod state;

/// flutter_rust_bridge 对外接口模块
pub mod api {
    use crate::comic_source::jmcomic::client::{CategorySort, FavoriteSort, SearchSort};
    use crate::config::Config;
    use crate::service::model::{
        AlbumDetailInfo, CategoryInfo, ChapterInfo, FavoriteInfo, PromoteListInfo, PromoteSectionInfo, SearchInfo,
        SerializationInfo, ToggleType, UserInfo,
    };
    use crate::state::AppState;

    /// 初始化后端：创建数据目录、加载配置并构建缓存与业务服务（应在 `RustLib.init()` 之后、其他 API 之前调用）
    pub async fn init_rust(path: String) -> anyhow::Result<bool> {
        AppState::init(path).await
    }

    /// 获取当前配置（需先调用 `init_rust`）
    pub async fn get_config() -> anyhow::Result<Config> {
        AppState::get_config().await
    }

    /// 更新配置并持久化到 `config.json`（域名/代理/并发数变化时自动重建客户端）
    pub async fn update_config(config: Config) -> anyhow::Result<()> {
        AppState::update_config(config).await
    }

    // ---------- JmService 透传（数据类带 API 缓存，图片类带磁盘缓存） ----------

    /// 登录，返回用户信息
    pub async fn login(username: String, password: String) -> anyhow::Result<UserInfo> {
        AppState::get_app()?.login(&username, &password).await
    }

    /// 收藏列表（不缓存）
    pub async fn get_favorite(folder_id: i64, page: i32, sort: FavoriteSort) -> anyhow::Result<FavoriteInfo> {
        AppState::get_app()?.get_favorite(folder_id, page, sort).await
    }

    /// 切换收藏（不缓存）
    pub async fn toggle_favorite(album_id: i64) -> anyhow::Result<ToggleType> {
        AppState::get_app()?.toggle_favorite(album_id).await
    }

    /// 专辑详情（API 缓存）
    pub async fn get_album(id: i64) -> anyhow::Result<AlbumDetailInfo> {
        AppState::get_app()?.get_album(id).await
    }

    /// 章节：id、图片名列表、scramble_id（API 缓存）
    pub async fn get_chapter(chapter_id: i64) -> anyhow::Result<ChapterInfo> {
        AppState::get_app()?.get_chapter(chapter_id).await
    }

    /// 首页推荐分组（API 缓存）
    pub async fn get_promote() -> anyhow::Result<Vec<PromoteSectionInfo>> {
        AppState::get_app()?.get_promote().await
    }

    /// 推荐分组下的分页专辑列表（API 缓存）
    pub async fn get_promote_list(id: i64, page: i32) -> anyhow::Result<PromoteListInfo> {
        AppState::get_app()?.get_promote_list(id, page).await
    }

    /// 每周连载更新：type 为 all/manga/hanman，date 为 0~7（0 表示全部，1-7 表示周一到周日）（API 缓存）
    pub async fn get_serialization(serial_type: String, date: String, page: i32) -> anyhow::Result<SerializationInfo> {
        AppState::get_app()?.get_serialization(&serial_type, &date, page).await
    }

    /// 搜索（API 缓存）
    pub async fn search(keyword: String, page: i32, sort: SearchSort) -> anyhow::Result<SearchInfo> {
        AppState::get_app()?.search(&keyword, page, sort).await
    }

    /// 分类列表（API 缓存）
    pub async fn get_categories() -> anyhow::Result<CategoryInfo> {
        AppState::get_app()?.get_categories().await
    }

    /// 分类下的专辑列表（API 缓存，结构与搜索一致）
    pub async fn get_categories_filter(category: String, page: i32, sort: CategorySort) -> anyhow::Result<SearchInfo> {
        AppState::get_app()?.get_categories_filter(&category, page, sort).await
    }

    /// 封面，返回磁盘路径字符串（`image_name` 可为 `{album_id}.jpg` 或 `{album_id}_3x4.jpg`）
    pub async fn get_cover(image_name: String) -> anyhow::Result<String> {
        AppState::get_app()?.get_cover(&image_name).await
    }

    /// 章节图片，返回磁盘路径字符串（已按 scramble_id 还原分块）
    pub async fn get_photo(scramble_id: i32, chapter_id: i32, image_name: String) -> anyhow::Result<String> {
        AppState::get_app()?
            .get_photo(scramble_id, chapter_id, &image_name)
            .await
    }
}
