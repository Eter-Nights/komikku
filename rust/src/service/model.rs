use crate::comic_source::jmcomic::client::APP_SCRAMBLE_ID;
use crate::comic_source::jmcomic::response::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserInfo {
    pub uid: i64,
    pub username: String,
    pub email: String,
    pub coin: i64,
    pub album_favorites: i64,
    pub album_favorites_max: i64,
    pub level_name: String,
    pub level: i64,
    pub next_level_exp: i64,
    pub exp: i64,
}

impl From<LoginResp> for UserInfo {
    fn from(resp: LoginResp) -> Self {
        Self {
            uid: resp.uid,
            username: resp.username,
            email: resp.email,
            coin: resp.coin,
            album_favorites: resp.album_favorites,
            album_favorites_max: resp.album_favorites_max,
            level_name: resp.level_name,
            level: resp.level,
            next_level_exp: resp.next_level_exp,
            exp: resp.exp.parse().unwrap_or_default(),
        }
    }
}

/// 专辑简要信息
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AlbumBriefInfo {
    pub id: i64,
    pub name: String,
    pub author: String,
}

impl From<AlbumItemResp> for AlbumBriefInfo {
    fn from(resp: AlbumItemResp) -> Self {
        Self {
            id: resp.id,
            name: resp.name,
            author: resp.author,
        }
    }
}

impl From<FavoriteItemResp> for AlbumBriefInfo {
    fn from(resp: FavoriteItemResp) -> Self {
        Self {
            id: resp.id,
            name: resp.name,
            author: resp.author,
        }
    }
}

impl From<RelatedListResp> for AlbumBriefInfo {
    fn from(resp: RelatedListResp) -> Self {
        Self {
            id: resp.id,
            name: resp.name,
            author: resp.author,
        }
    }
}

impl From<AlbumResp> for AlbumBriefInfo {
    fn from(resp: AlbumResp) -> Self {
        Self {
            id: resp.id,
            name: resp.name,
            author: resp.author.join(" "),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FavoriteInfo {
    pub total: i64,
    pub count: i64,
    pub list: Vec<AlbumBriefInfo>,
}

impl From<FavoriteResp> for FavoriteInfo {
    fn from(resp: FavoriteResp) -> Self {
        Self {
            total: resp.total,
            count: resp.count,
            list: resp.list.into_iter().map(AlbumBriefInfo::from).collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ToggleType {
    Add,
    Remove,
}

impl From<ToggleFavoriteResp> for ToggleType {
    fn from(resp: ToggleFavoriteResp) -> Self {
        match resp.toggle_type.as_str() {
            "remove" => Self::Remove,
            _ => Self::Add,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SeriesInfo {
    pub id: i64,
    pub name: String,
    pub sort: String,
}

impl From<SeriesResp> for SeriesInfo {
    fn from(resp: SeriesResp) -> Self {
        Self {
            id: resp.id,
            name: resp.name,
            sort: resp.sort,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AlbumDetailInfo {
    pub id: i64,
    pub name: String,
    pub author: Vec<String>,
    pub description: String,
    pub tags: Vec<String>,
    pub total_photos: i64,
    pub addtime: String,
    pub total_views: i64,
    pub likes: i64,
    pub series: Vec<SeriesInfo>,
    pub comment_total: i64,
    pub liked: bool,
    pub is_favorite: bool,
}

impl From<AlbumResp> for AlbumDetailInfo {
    fn from(resp: AlbumResp) -> Self {
        Self {
            id: resp.id,
            name: resp.name,
            author: resp.author,
            description: resp.description,
            tags: resp.tags,
            total_photos: resp.total_photos,
            addtime: resp.addtime,
            total_views: resp.total_views,
            likes: resp.likes,
            series: build_series(resp.id, resp.series),
            comment_total: resp.comment_total,
            liked: resp.liked,
            is_favorite: resp.is_favorite,
        }
    }
}

/// 列表为空时用传入的 id 构造一个「第1话」条目
fn build_series(id: i64, series: Vec<SeriesResp>) -> Vec<SeriesInfo> {
    let mut list: Vec<SeriesInfo> = series
        .into_iter()
        .map(|s| {
            let mut info = SeriesInfo::from(s);
            if info.name.is_empty() {
                info.name = format!("第{}话", info.sort);
            }
            info
        })
        .collect();

    if list.is_empty() {
        list.push(SeriesInfo {
            id,
            name: "第1话".to_string(),
            sort: "1".to_string(),
        });
    }

    list
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChapterInfo {
    pub id: i64,
    pub scramble_id: i32,
    pub images: Vec<String>,
}

impl From<ChapterResp> for ChapterInfo {
    fn from(resp: ChapterResp) -> Self {
        Self {
            id: resp.id,
            scramble_id: APP_SCRAMBLE_ID,
            images: resp.images,
        }
    }
}

/// 首页推荐分组（promote 接口返回，content 内嵌专辑列表）
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PromoteSectionInfo {
    pub id: i64,
    pub title: String,
    pub slug: String,
    /// 分组类型：promote / category_id / not_in_category_id / library / novels
    pub section_type: String,
    pub content: Vec<AlbumBriefInfo>,
}

impl From<PromoteSectionResp> for PromoteSectionInfo {
    fn from(resp: PromoteSectionResp) -> Self {
        Self {
            id: resp.id,
            title: resp.title,
            slug: resp.slug,
            section_type: resp.section_type,
            content: resp.content.into_iter().map(AlbumBriefInfo::from).collect(),
        }
    }
}

/// 推荐分组下的分页专辑列表（promote_list 接口）
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PromoteListInfo {
    pub total: i64,
    pub list: Vec<AlbumBriefInfo>,
}

impl From<PromoteListResp> for PromoteListInfo {
    fn from(resp: PromoteListResp) -> Self {
        Self {
            total: resp.total,
            list: resp.list.into_iter().map(AlbumBriefInfo::from).collect(),
        }
    }
}

/// 每周连载更新（serialization 接口，返回分页专辑列表）
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SerializationInfo {
    pub list: Vec<AlbumBriefInfo>,
}

impl From<SerializationResp> for SerializationInfo {
    fn from(resp: SerializationResp) -> Self {
        Self {
            list: resp.list.into_iter().map(AlbumBriefInfo::from).collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchInfo {
    pub search_query: String,
    pub total: i64,
    pub content: Vec<AlbumBriefInfo>,
}

impl From<SearchResp> for SearchInfo {
    fn from(resp: SearchResp) -> Self {
        Self {
            search_query: resp.search_query,
            total: resp.total,
            // content 为空则映射空向量
            content: resp.content.into_iter().map(AlbumBriefInfo::from).collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CategorySubInfo {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

impl From<CategorySubResp> for CategorySubInfo {
    fn from(resp: CategorySubResp) -> Self {
        Self {
            id: resp.cid,
            name: resp.name,
            slug: resp.slug,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CategoryItemInfo {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub total_albums: i64,
    // 旧缓存数据可能没有该字段，缺失时默认为空列表
    #[serde(default)]
    pub sub_categories: Vec<CategorySubInfo>,
}

impl From<CategoryItemResp> for CategoryItemInfo {
    fn from(resp: CategoryItemResp) -> Self {
        Self {
            id: resp.id,
            name: resp.name,
            slug: resp.slug,
            total_albums: resp.total_albums,
            sub_categories: resp.sub_categories.into_iter().map(CategorySubInfo::from).collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CategoryBlockInfo {
    pub title: String,
    pub content: Vec<String>,
}

impl From<CategoryBlockResp> for CategoryBlockInfo {
    fn from(resp: CategoryBlockResp) -> Self {
        Self {
            title: resp.title,
            content: resp.content,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CategoryInfo {
    pub categories: Vec<CategoryItemInfo>,
    pub blocks: Vec<CategoryBlockInfo>,
}

impl From<CategoryResp> for CategoryInfo {
    fn from(resp: CategoryResp) -> Self {
        Self {
            categories: resp.categories.into_iter().map(CategoryItemInfo::from).collect(),
            blocks: resp.blocks.into_iter().map(CategoryBlockInfo::from).collect(),
        }
    }
}
