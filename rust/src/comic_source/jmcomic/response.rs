use serde::Deserialize;
use serde_aux::field_attributes::{
    deserialize_number_from_string, deserialize_option_number_from_string, deserialize_string_from_number,
};

#[derive(Debug, Deserialize)]
pub struct JmcomicResp {
    pub code: i32,
    pub data: serde_json::Value,
    #[serde(default)]
    pub error_msg: String,
}

#[derive(Debug, Deserialize)]
pub struct BadgeResp {
    pub content: String,
    pub name: String,
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginResp {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub uid: i64,
    pub username: String,
    pub email: String,
    pub emailverified: String,
    pub photo: String,
    pub fname: String,
    pub gender: String,
    pub message: String,
    // 禁漫 API 的 coin 有时返回数字(3339)、有时返回字符串("3339")
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub coin: i64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub album_favorites: i64,
    pub s: String,
    pub level_name: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub level: i64,
    #[serde(rename = "nextLevelExp", deserialize_with = "deserialize_number_from_string")]
    pub next_level_exp: i64,
    pub exp: String,
    #[serde(rename = "expPercent")]
    pub exp_percent: f64,
    pub badges: Vec<BadgeResp>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub album_favorites_max: i64,
    pub ad_free: bool,
    pub ad_free_before: String,
    pub charge: String,
    pub jar: String,
    pub invitation_qrcode: String,
    pub invitation_url: String,
    pub invited_cnt: String,
    pub jwttoken: String,
}

#[derive(Debug, Deserialize)]
pub struct FavoriteItemResp {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub id: i64,
    pub author: String,
    pub description: Option<String>,
    pub name: String,
    pub latest_ep: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub latest_ep_aid: Option<i64>,
    pub image: String,
    pub category: CategoryDataResp,
    pub category_sub: CategoryDataResp,
}

#[derive(Debug, Deserialize)]
pub struct FavoriteFolderResp {
    #[serde(rename = "FID", deserialize_with = "deserialize_number_from_string")]
    pub fid: i64,
    #[serde(rename = "UID", deserialize_with = "deserialize_number_from_string")]
    pub uid: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct FavoriteResp {
    pub list: Vec<FavoriteItemResp>,
    pub folder_list: Vec<FavoriteFolderResp>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub total: i64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub count: i64,
}

#[derive(Debug, Deserialize)]
pub struct ToggleFavoriteResp {
    pub status: String,
    pub msg: String,
    #[serde(rename = "type")]
    pub toggle_type: String,
}

#[derive(Debug, Deserialize)]
pub struct SeriesResp {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub id: i64,
    pub name: String,
    pub sort: String,
}

#[derive(Debug, Deserialize)]
pub struct RelatedListResp {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub id: i64,
    pub author: String,
    pub name: String,
    pub image: String,
}

#[derive(Debug, Deserialize)]
pub struct AlbumResp {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub id: i64,
    pub name: String,
    pub images: Vec<String>,
    pub addtime: String,
    pub description: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub total_views: i64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub total_photos: i64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub likes: i64,
    pub series: Vec<SeriesResp>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub series_id: i64,
    pub real_link: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub comment_total: i64,
    pub author: Vec<String>,
    pub tags: Vec<String>,
    pub works: Vec<String>,
    pub actors: Vec<String>,
    pub related_list: Vec<RelatedListResp>,
    pub liked: bool,
    pub is_favorite: bool,
    pub is_aids: bool,
    pub price: String,
    pub purchased: String,
}

#[derive(Debug, Deserialize)]
pub struct ChapterResp {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub id: i64,
    pub series: Vec<SeriesResp>,
    pub tags: String,
    pub name: String,
    pub images: Vec<String>,
    pub addtime: String,
    pub real_link: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub series_id: i64,
    pub is_favorite: bool,
    pub liked: bool,
}

/// 专辑条目 description 要么不存在，要么为null，所以放弃这个参数
#[derive(Debug, Deserialize)]
pub struct AlbumItemResp {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub id: i64,
    pub author: String,
    pub name: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub category: Option<CategoryDataResp>,
    #[serde(default)]
    pub category_sub: Option<CategoryDataResp>,
    #[serde(default)]
    pub liked: bool,
    // 兼容两种字段名：搜索/分类等返回 is_favorite，serialization 返回 favorite
    #[serde(default, alias = "favorite")]
    pub is_favorite: bool,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub update_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct PromoteSectionResp {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub id: i64,
    pub title: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default, rename = "type")]
    pub section_type: String,
    #[serde(default, deserialize_with = "deserialize_string_from_number")]
    pub filter_val: String,
    #[serde(default)]
    pub content: Vec<AlbumItemResp>,
}

#[derive(Debug, Deserialize)]
pub struct PromoteListResp {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub total: i64,
    pub list: Vec<AlbumItemResp>,
}

/// 连载（serialization）分页响应：`{"list": [...]}`，条目复用 AlbumItemResp（is_favorite 已兼容 favorite 字段名）。
#[derive(Debug, Deserialize)]
pub struct SerializationResp {
    #[serde(default)]
    pub list: Vec<AlbumItemResp>,
}

#[derive(Debug, Deserialize)]
pub struct CategoryDataResp {
    pub id: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResp {
    pub search_query: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub total: i64,
    // 搜索禁漫号返回该字段，关键词不返回
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub redirect_aid: Option<i64>,
    pub content: Vec<AlbumItemResp>,
}

#[derive(Debug, Deserialize)]
pub struct CategorySubResp {
    #[serde(rename = "CID", deserialize_with = "deserialize_number_from_string")]
    pub cid: i64,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Deserialize)]
pub struct CategoryItemResp {
    // id/total_albums 可能是数字或字符串
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub id: i64,
    pub name: String,
    pub slug: String,
    #[serde(default, rename = "type")]
    pub category_type: Option<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub total_albums: i64,
    #[serde(default)]
    pub sub_categories: Vec<CategorySubResp>,
}

#[derive(Debug, Deserialize)]
pub struct CategoryBlockResp {
    pub title: String,
    pub content: Vec<String>,
}

/// categories 接口：分类列表 + 标签块
#[derive(Debug, Deserialize)]
pub struct CategoryResp {
    pub categories: Vec<CategoryItemResp>,
    pub blocks: Vec<CategoryBlockResp>,
}
