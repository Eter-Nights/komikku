//! 业务服务层：数据格式转换 + 部分业务逻辑（持有统一请求客户端 `JMClient`）。

use super::model::{
    AlbumBriefInfo, AlbumDetailInfo, CategoryInfo, ChapterInfo, FavoriteInfo, PromoteListInfo, PromoteSectionInfo,
    SearchInfo, SerializationInfo, ToggleType, UserInfo,
};
use crate::cache::{ApiCache, ImageCache};
use crate::comic_source::jmcomic::client::{CategorySort, FavoriteSort, JMClient, SearchSort};
use crate::config::Config;

use std::sync::Arc;

/// 业务服务：格式转换 + 业务逻辑 + 缓存（缓存实例由上层注入共享）。
///
/// 缓存逻辑内聚在本服务（自管 key/序列化）；缓存 key 必须带 `jm:` 前缀避免多源冲突。
pub struct JmService {
    /// 统一请求客户端
    client: Arc<JMClient>,
    /// API 缓存（共享实例，key 带 `jm:` 前缀）
    api_cache: Arc<ApiCache>,
    /// 图片缓存（共享实例，key 带 `jm:` 前缀）
    image_cache: Arc<ImageCache>,
}

impl JmService {
    /// 由配置与共享缓存构建业务服务
    pub fn new(config: &Config, api_cache: Arc<ApiCache>, image_cache: Arc<ImageCache>) -> Self {
        Self {
            client: Arc::new(JMClient::new(config)),
            api_cache,
            image_cache,
        }
    }

    /// 按新配置重建内层客户端（保留 cookie jar，登录态不丢）
    pub fn reload(&self, config: &Config) {
        self.client.reload(config);
    }

    // ---------- 对外业务函数（数据）：格式转换 + 部分业务逻辑 ----------

    /// 登录，返回用户信息
    pub async fn login(&self, username: &str, password: &str) -> anyhow::Result<UserInfo> {
        Ok(self.client.request_login(username, password).await?.into())
    }

    /// 获取收藏列表（不缓存）
    pub async fn get_favorite(&self, folder_id: i64, page: i32, sort: FavoriteSort) -> anyhow::Result<FavoriteInfo> {
        Ok(self.client.request_favorite(folder_id, page, sort).await?.into())
    }

    /// 切换收藏（收藏或取消收藏，JM 的收藏和取消收藏是一个接口）
    pub async fn toggle_favorite(&self, album_id: i64) -> anyhow::Result<ToggleType> {
        Ok(self.client.request_toggle_favorite(album_id).await?.into())
    }

    /// 获取专辑详情（API 缓存）
    pub async fn get_album(&self, id: i64) -> anyhow::Result<AlbumDetailInfo> {
        let key = format!("jm:album:{id}");
        let json = self
            .api_cache
            .get(&key, || {
                let client = &self.client;
                async move {
                    let info = client.request_album(id).await?;
                    Ok(serde_json::to_string(&AlbumDetailInfo::from(info))?)
                }
            })
            .await?;
        Ok(serde_json::from_str(&json)?)
    }

    /// 获取章节：返回 id、图片名列表和图片解密所需的 scramble_id（API 缓存）
    pub async fn get_chapter(&self, chapter_id: i64) -> anyhow::Result<ChapterInfo> {
        let key = format!("jm:chapter:{chapter_id}");
        let json = self
            .api_cache
            .get(&key, || {
                let client = &self.client;
                async move {
                    let chapter = client.request_chapter(chapter_id).await?;
                    // 先用默认 scramble_id，再用真实值覆盖
                    let mut info = ChapterInfo::from(chapter);
                    info.scramble_id = client.request_scramble_id(chapter_id).await?;
                    Ok(serde_json::to_string(&info)?)
                }
            })
            .await?;
        Ok(serde_json::from_str(&json)?)
    }

    /// 首页推荐分组（API 缓存）
    pub async fn get_promote(&self) -> anyhow::Result<Vec<PromoteSectionInfo>> {
        let key = "jm:promote".to_string();
        let json = self
            .api_cache
            .get(&key, || {
                let client = &self.client;
                async move {
                    let sections = client.request_promote().await?;
                    let infos: Vec<PromoteSectionInfo> = sections.into_iter().map(PromoteSectionInfo::from).collect();
                    Ok(serde_json::to_string(&infos)?)
                }
            })
            .await?;
        Ok(serde_json::from_str(&json)?)
    }

    /// 推荐分组下的分页专辑列表（API 缓存）
    pub async fn get_promote_list(&self, id: i64, page: i32) -> anyhow::Result<PromoteListInfo> {
        let key = format!("jm:promote_list:{id}:{page}");
        let json = self
            .api_cache
            .get(&key, || {
                let client = &self.client;
                async move {
                    let resp = client.request_promote_list(id, page).await?;
                    Ok(serde_json::to_string(&PromoteListInfo::from(resp))?)
                }
            })
            .await?;
        Ok(serde_json::from_str(&json)?)
    }

    /// 每周连载更新：type 为 all/manga/hanman，date 为 0~7（0 表示全部，1-7 表示周一到周日）（API 缓存）
    pub async fn get_serialization(
        &self,
        serial_type: &str,
        date: &str,
        page: i32,
    ) -> anyhow::Result<SerializationInfo> {
        let key = format!("jm:serialization:{serial_type}:{date}:{page}");
        let serial_type = serial_type.to_string();
        let date = date.to_string();
        let json = self
            .api_cache
            .get(&key, move || {
                let client = &self.client;
                let serial_type = serial_type.clone();
                let date = date.clone();
                async move {
                    let resp = client.request_serialization(&serial_type, &date, page).await?;
                    Ok(serde_json::to_string(&SerializationInfo::from(resp))?)
                }
            })
            .await?;
        Ok(serde_json::from_str(&json)?)
    }

    /// 搜索：命中禁漫号（redirect_aid）时拉取专辑详情作为单条结果（API 缓存）
    pub async fn search(&self, keyword: &str, page: i32, sort: SearchSort) -> anyhow::Result<SearchInfo> {
        let key = format!("jm:search:{keyword}:{page}:{}", sort.as_ref());
        let keyword = keyword.to_string();
        let json = self
            .api_cache
            .get(&key, move || {
                let client = &self.client;
                let keyword = keyword.clone();
                async move {
                    let resp = client.request_search(&keyword, page, sort).await?;
                    // 有重定向 id 说明精确命中专辑，拉取专辑详情作为单条结果
                    if let Some(album_id) = resp.redirect_aid {
                        let album = client.request_album(album_id).await?;
                        return Ok(serde_json::to_string(&SearchInfo {
                            search_query: resp.search_query,
                            total: resp.total,
                            content: vec![AlbumBriefInfo::from(album)],
                        })?);
                    }
                    let info: SearchInfo = resp.into();
                    Ok(serde_json::to_string(&info)?)
                }
            })
            .await?;
        Ok(serde_json::from_str(&json)?)
    }

    /// 获取分类列表（API 缓存）
    pub async fn get_categories(&self) -> anyhow::Result<CategoryInfo> {
        let key = "jm:categories".to_string();
        let json = self
            .api_cache
            .get(&key, || {
                let client = &self.client;
                async move {
                    let resp = client.request_categories().await?;
                    Ok(serde_json::to_string(&CategoryInfo::from(resp))?)
                }
            })
            .await?;
        Ok(serde_json::from_str(&json)?)
    }

    /// 获取分类下的专辑列表（结构与搜索一致，API 缓存）
    pub async fn get_categories_filter(
        &self,
        category: &str,
        page: i32,
        sort: CategorySort,
    ) -> anyhow::Result<SearchInfo> {
        let key = format!("jm:categories_filter:{category}:{page}:{}", sort.as_ref());
        let category = category.to_string();
        let json = self
            .api_cache
            .get(&key, move || {
                let client = &self.client;
                let category = category.clone();
                async move {
                    let resp = client.request_categories_filter(&category, page, sort).await?;
                    Ok(serde_json::to_string(&SearchInfo::from(resp))?)
                }
            })
            .await?;
        Ok(serde_json::from_str(&json)?)
    }

    // ---------- 对外业务函数（图片）：磁盘缓存，返回文件路径字符串 ----------

    /// 封面，返回磁盘路径字符串（`image_name` 可为 `{album_id}.jpg` 原图或 `{album_id}_3x4.jpg` 缩略图）
    pub async fn get_cover(&self, image_name: &str) -> anyhow::Result<String> {
        let key = format!("jm:cover:{image_name}");
        let image_name = image_name.to_string();
        self.image_cache
            .get(&key, move || {
                let client = &self.client;
                let image_name = image_name.clone();
                async move { client.request_cover(&image_name).await }
            })
            .await
    }

    /// 章节图片，返回磁盘路径字符串（已按 scramble_id 还原打乱分块）
    pub async fn get_photo(&self, scramble_id: i32, chapter_id: i32, image_name: &str) -> anyhow::Result<String> {
        let key = format!("jm:photo:{chapter_id}:{image_name}");
        let image_name = image_name.to_string();
        self.image_cache
            .get(&key, move || {
                let client = &self.client;
                let image_name = image_name.clone();
                async move { client.request_photo(scramble_id, chapter_id, &image_name).await }
            })
            .await
    }
}
