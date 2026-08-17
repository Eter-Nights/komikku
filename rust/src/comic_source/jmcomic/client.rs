//! 统一请求客户端：向禁漫服务器发起 API 请求（Token、解密、重试）与图片请求（封面 + 章节图）。

use super::response::*;
use crate::config::{Config, ProxyMode};

use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockModeDecrypt, KeyInit};
use aes::Aes256;
use anyhow::Context;
use base64::{engine::general_purpose, Engine};
use ecb::Decryptor;
use parking_lot::RwLock;
use reqwest::cookie::Jar;
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::{policies::ExponentialBackoff, Jitter, RetryTransientMiddleware};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use strum::AsRefStr;
use tokio::sync::Semaphore;

const USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 16; PTP-AN10 Build/HONORPTP-AN10) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.7204.179 Mobile Safari/537.36";
const APP_VERSION: &str = "2.0.17";
const APP_TOKEN_SECRET: &str = "18comicAPP";
const APP_TOKEN_SECRET_2: &str = "18comicAPPContent";
const APP_DATA_SECRET: &str = "185Hcomic3PAPP7R";
pub const APP_SCRAMBLE_ID: i32 = 220980;

/// API 域名，方法直接返回对应的域名
#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize, AsRefStr)]
pub enum ApiDomain {
    #[default]
    #[strum(serialize = "https://www.cdnhth.cc")]
    Domain1,
    #[strum(serialize = "https://www.cdnzack.cc")]
    Domain2,
    #[strum(serialize = "https://www.cdnhth.net")]
    Domain3,
    #[strum(serialize = "https://www.cdnbea.net")]
    Domain4,
    #[strum(serialize = "https://www.cdn-mspjmapiproxy.xyz")]
    Domain5,
}

/// 图片域名，方法直接返回对应的域名
#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize, AsRefStr)]
pub enum ImageDomain {
    #[default]
    #[strum(serialize = "https://cdn-msp.jmapiproxy1.cc")]
    Domain1,
    #[strum(serialize = "https://cdn-msp.jmapiproxy2.cc")]
    Domain2,
    #[strum(serialize = "https://cdn-msp.jmapiproxy3.cc")]
    Domain3,
    #[strum(serialize = "https://cdn-msp.jmapinodeudzn.net")]
    Domain4,
    #[strum(serialize = "https://cdn-msp.jmdanjonproxy.xyz")]
    Domain5,
}

#[derive(PartialEq, AsRefStr)]
pub enum Url {
    // API 接口
    #[strum(serialize = "/login")]
    Login,
    #[strum(serialize = "/favorite")]
    Favorite,
    #[strum(serialize = "/album")]
    Album,
    #[strum(serialize = "/chapter")]
    Chapter,
    #[strum(serialize = "/promote")]
    Promote,
    #[strum(serialize = "/promote_list")]
    PromoteList,
    #[strum(serialize = "/serialization")]
    Serialization,
    #[strum(serialize = "/search")]
    Search,
    #[strum(serialize = "/categories")]
    Categories,
    #[strum(serialize = "/categories/filter")]
    CategoriesFilter,
    #[strum(serialize = "/chapter_view_template")]
    ScrambleId,
    // 图片接口
    #[strum(serialize = "/media/albums")]
    MediaAlbums,
    #[strum(serialize = "/media/photos")]
    MediaPhotos,
}

/// 搜索排序
#[derive(Clone, Debug, Serialize, Deserialize, AsRefStr)]
pub enum SearchSort {
    /// 最新
    #[strum(serialize = "mr")]
    Latest,
    /// 最多点击
    #[strum(serialize = "mv")]
    View,
    /// 最多图片
    #[strum(serialize = "mp")]
    Picture,
    /// 最多喜欢
    #[strum(serialize = "tf")]
    Like,
}

/// 分类页排序（categories/filter）
#[derive(Clone, Debug, Serialize, Deserialize, AsRefStr)]
pub enum CategorySort {
    /// 最新
    #[strum(serialize = "mr")]
    Latest,
    /// 最多喜欢
    #[strum(serialize = "tf")]
    Like,
    /// 总排名
    #[strum(serialize = "mv")]
    TotalRanking,
    /// 月排名
    #[strum(serialize = "mv_m")]
    MonthRanking,
}

#[derive(Clone, Debug, Serialize, Deserialize, AsRefStr)]
pub enum FavoriteSort {
    #[strum(serialize = "mr")]
    FavoriteTime,
    #[strum(serialize = "mp")]
    UpdateTime,
}

/// 统一请求客户端：API 请求 + 图片请求（共享 cookie，图片侧带并发限流）
pub struct JMClient {
    /// 共享 cookie 容器：登录态在 reload 后依然保留
    jar: Arc<Jar>,
    /// API 域名（reload 时更新）
    api_domain: RwLock<ApiDomain>,
    /// API 客户端（reload 时替换）
    api_client: RwLock<ClientWithMiddleware>,
    /// 图片域名（reload 时更新）
    img_domain: RwLock<ImageDomain>,
    /// 图片客户端（reload 时替换）
    img_client: RwLock<ClientWithMiddleware>,
    /// 图片下载并发信号量（全局共享，跨图片限流；reload 时替换内层 Arc，旧 permit 持旧 Arc 安全）
    img_sem: RwLock<Arc<Semaphore>>,
}

impl JMClient {
    /// 由配置构建 API 与图片请求客户端
    pub fn new(config: &Config) -> Self {
        let jar = Arc::new(Jar::default());
        Self {
            jar: jar.clone(),
            api_domain: RwLock::new(config.api_domain.clone()),
            api_client: RwLock::new(build_api_client(config, jar)),
            img_domain: RwLock::new(config.image_domain.clone()),
            img_client: RwLock::new(build_img_client(config)),
            img_sem: RwLock::new(Arc::new(Semaphore::new(config.img_concurrency as usize))),
        }
    }

    /// 按新配置重建内层客户端（保留 cookie jar，登录态不丢；并发数变化重建信号量）
    pub fn reload(&self, config: &Config) {
        *self.api_domain.write() = config.api_domain.clone();
        *self.api_client.write() = build_api_client(config, self.jar.clone());
        *self.img_domain.write() = config.image_domain.clone();
        *self.img_client.write() = build_img_client(config);
        // 替换内层信号量使并发数即时生效（旧 permit 持旧 Arc，安全）
        *self.img_sem.write() = Arc::new(Semaphore::new(config.img_concurrency as usize));
    }

    /// 登录
    pub async fn request_login(&self, username: &str, password: &str) -> anyhow::Result<LoginResp> {
        let params = serde_json::json!({
            "username": username,
            "password": password,
        });

        let data = self.request_api(reqwest::Method::POST, Url::Login, params).await?;

        Ok(serde_json::from_str::<LoginResp>(&data)?)
    }

    /// 收藏列表
    pub async fn request_favorite(
        &self,
        folder_id: i64,
        page: i32,
        sort: FavoriteSort,
    ) -> anyhow::Result<FavoriteResp> {
        let params = serde_json::json!({
            "folder_id": folder_id,
            "page": page,
            "o": sort.as_ref(),
        });

        let data = self.request_api(reqwest::Method::GET, Url::Favorite, params).await?;

        Ok(serde_json::from_str::<FavoriteResp>(&data)?)
    }

    /// 切换收藏（收藏/取消收藏，JM 的收藏和取消收藏是一个接口）
    pub async fn request_toggle_favorite(&self, album_id: i64) -> anyhow::Result<ToggleFavoriteResp> {
        let params = serde_json::json!({
            "aid": album_id,
        });

        let data = self.request_api(reqwest::Method::POST, Url::Favorite, params).await?;

        Ok(serde_json::from_str::<ToggleFavoriteResp>(&data)?)
    }

    /// 专辑详情
    pub async fn request_album(&self, id: i64) -> anyhow::Result<AlbumResp> {
        let params = serde_json::json!({
            "id": id,
        });

        let data = self.request_api(reqwest::Method::GET, Url::Album, params).await?;

        Ok(serde_json::from_str::<AlbumResp>(&data)?)
    }

    /// 章节详情
    pub async fn request_chapter(&self, id: i64) -> anyhow::Result<ChapterResp> {
        let params = serde_json::json!({
            "id": id,
        });

        let data = self.request_api(reqwest::Method::GET, Url::Chapter, params).await?;

        Ok(serde_json::from_str::<ChapterResp>(&data)?)
    }

    /// 首页推荐：promote 接口返回推荐分组列表（也可能返回单个对象，统一归一为数组）
    pub async fn request_promote(&self) -> anyhow::Result<Vec<PromoteSectionResp>> {
        let time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let params = serde_json::json!({
            "_": time,
        });

        let data = self.request_api(reqwest::Method::GET, Url::Promote, params).await?;

        // 兼容数组 / 单个对象两种返回形式
        let value: serde_json::Value = serde_json::from_str(&data)?;
        match value {
            serde_json::Value::Array(_) => Ok(serde_json::from_value(value)?),
            serde_json::Value::Object(_) => Ok(vec![serde_json::from_value(value)?]),
            _ => Ok(vec![]),
        }
    }

    /// 推荐分组下的分页专辑列表
    pub async fn request_promote_list(&self, id: i64, page: i32) -> anyhow::Result<PromoteListResp> {
        let params = serde_json::json!({
            "id": id,
            "page": page,
        });

        let data = self.request_api(reqwest::Method::GET, Url::PromoteList, params).await?;

        Ok(serde_json::from_str::<PromoteListResp>(&data)?)
    }

    /// 每周连载更新：type 为 all/manga/hanman，date 为 0~7（0 表示全部，1-7 表示周一到周日）
    pub async fn request_serialization(
        &self,
        date: &str,
        serial_type: &str,
        page: i32,
    ) -> anyhow::Result<SerializationResp> {
        let params = serde_json::json!({
            "date": date,
            "type": serial_type,
            "page": page,
        });

        let data = self
            .request_api(reqwest::Method::GET, Url::Serialization, params)
            .await?;

        Ok(serde_json::from_str::<SerializationResp>(&data)?)
    }

    /// 搜索
    pub async fn request_search(&self, keyword: &str, page: i32, sort: SearchSort) -> anyhow::Result<SearchResp> {
        let params = serde_json::json!({
            "search_query": keyword,
            "page": page,
            "o": sort.as_ref(),
        });

        let data = self.request_api(reqwest::Method::GET, Url::Search, params).await?;

        Ok(serde_json::from_str::<SearchResp>(&data)?)
    }

    /// 分类列表
    pub async fn request_categories(&self) -> anyhow::Result<CategoryResp> {
        let data = self
            .request_api(reqwest::Method::GET, Url::Categories, serde_json::json!({}))
            .await?;

        Ok(serde_json::from_str::<CategoryResp>(&data)?)
    }

    /// 分类下的专辑列表（响应结构与搜索一致，复用 SearchResp）
    pub async fn request_categories_filter(
        &self,
        category: &str,
        page: i32,
        sort: CategorySort,
    ) -> anyhow::Result<SearchResp> {
        let params = serde_json::json!({
            "c": category,
            "o": sort.as_ref(),
            "page": page,
        });

        let data = self
            .request_api(reqwest::Method::GET, Url::CategoriesFilter, params)
            .await?;

        Ok(serde_json::from_str::<SearchResp>(&data)?)
    }

    /// 图片解密所需的 scramble id（接口返回 HTML，解析其中 `var scramble_id = ...`）
    pub async fn request_scramble_id(&self, id: i64) -> anyhow::Result<i32> {
        let time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let params = serde_json::json!({
            "id": id,
            "v": time,
            "mode": "vertical",
            "page": 0,
            "app_img_shunt": 1,
            "express": "off",
        });

        let data = self.request_api(reqwest::Method::GET, Url::ScrambleId, params).await?;

        let scramble_id = data
            .split("var scramble_id = ")
            .nth(1)
            .and_then(|s| s.split(';').next())
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(APP_SCRAMBLE_ID);

        Ok(scramble_id)
    }

    /// 封面（源站）：获取并发许可后请求字节；`image_name` 可为原图或 `_3x4.jpg` 缩略图
    pub async fn request_cover(&self, image_name: &str) -> anyhow::Result<Vec<u8>> {
        // 先克隆 Arc 再 await，读锁不跨 await 存活
        let sem = self.img_sem.read().clone();
        let _permit = sem.acquire_owned().await.context("获取图片下载许可失败")?;

        let url = {
            let domain = self.img_domain.read();
            format!("{}{}/{}", domain.as_ref(), Url::MediaAlbums.as_ref(), image_name)
        };

        self.request_img(&url).await
    }

    /// 章节图片（源站）：先获取并发许可（排队限流），再请求原始字节并按 scramble_id 还原打乱分块
    pub async fn request_photo(&self, scramble_id: i32, chapter_id: i32, image_name: &str) -> anyhow::Result<Vec<u8>> {
        // 获取并发许可，持有至整个下载/还原完成
        let sem = self.img_sem.read().clone();
        let _permit = sem.acquire_owned().await.context("获取图片下载许可失败")?;

        let url = {
            let domain = self.img_domain.read();
            format!(
                "{}{}/{}/{}",
                domain.as_ref(),
                Url::MediaPhotos.as_ref(),
                chapter_id,
                image_name
            )
        };

        let bytes = self.request_img(&url).await?;

        // GIF 不打乱分块，直接返回
        let format = image::guess_format(&bytes).context("猜测图片格式失败")?;
        if format == image::ImageFormat::Gif {
            return Ok(bytes);
        }

        // 分块数为 0 表示未打乱，直接返回
        let block_num = self.calculate_block_num(scramble_id, chapter_id, image_name);
        if block_num == 0 {
            return Ok(bytes);
        }

        // 解码 → 还原分块 → 重新编码
        let img = image::load_from_memory_with_format(&bytes, format)?;
        let rgb = img.to_rgb8();
        let stitched = self.stitch_img(&rgb, block_num);

        let mut out = Vec::new();
        let dyn_img = image::DynamicImage::ImageRgb8(stitched);
        dyn_img
            .write_to(&mut std::io::Cursor::new(&mut out), format)
            .context("重新编码拼接图片失败")?;
        Ok(out)
    }

    // 统一api请求，图片请求和解密

    async fn request_api(
        &self,
        method: reqwest::Method,
        path: Url,
        params: serde_json::Value,
    ) -> anyhow::Result<String> {
        // 计算Token和Tokenparam头
        let time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let tokenparam = format!("{time},{APP_VERSION}");

        // ScrambleId接口使用不同的Token计算方式
        let token = if path == Url::ScrambleId {
            format!("{:x}", md5::compute(format!("{time}{APP_TOKEN_SECRET_2}")))
        } else {
            format!("{:x}", md5::compute(format!("{time}{APP_TOKEN_SECRET}")))
        };

        // 构造请求：读锁内拼 URL 构造 RequestBuilder，块结束释放锁
        let http_req = {
            let domain = self.api_domain.read();
            let client = self.api_client.read();
            let url = format!("{}{}", domain.as_ref(), path.as_ref());
            client
                .request(method.clone(), url)
                .header("Tokenparam", tokenparam)
                .header("Token", token)
        };

        // 发送请求
        let http_resp = if method == reqwest::Method::GET {
            http_req.query(&params).send().await
        } else {
            http_req.form(&params).send().await
        }
        .context(format!("发送请求失败: {} {}", method, path.as_ref()))?;

        let status = http_resp.status();
        let text = http_resp.text().await?;
        // 检查HTTP响应状态码
        if status != reqwest::StatusCode::OK {
            return Err(anyhow::anyhow!("请求失败，状态码：{} {}", status, text));
        }

        // ScrambleId接口直接返回HTML，不需要解析
        if path == Url::ScrambleId {
            Ok(text)
        } else {
            self.parse_api_response(time, &text).await
        }
    }

    async fn parse_api_response(&self, time: u64, text: &str) -> anyhow::Result<String> {
        // 解析响应体
        let jm_resp = serde_json::from_str::<JmcomicResp>(&text).context(format!("将body解析为JmResp失败{}", text))?;

        if jm_resp.code != 200 {
            tracing::error!("error_msg: {}", jm_resp.error_msg);
            return Err(anyhow::anyhow!("请求失败：{:#?}", jm_resp));
        }

        // 解密data字段
        let data = jm_resp
            .data
            .as_str()
            .context(format!("data 字段不是字符串: {}", jm_resp.data))?;

        self.decrypt_data(
            &format!("{:x}", md5::compute(format!("{}{}", time, APP_DATA_SECRET))),
            data,
        )
    }

    /// 图片请求：若返回为空说明 JM 那边缓存失效，携带时间戳再次请求以绕过缓存
    async fn request_img(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        let mut data = bytes::Bytes::new();

        for attempt in 0..2 {
            let mut request = {
                let client = self.img_client.read();
                client.get(url)
            };
            if attempt > 0 {
                // 带时间戳绕过失效缓存
                let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
                request = request.query(&[("ts", ts)]);
            }

            let http_resp = request
                .send()
                .await
                .context(format!("发送图片请求失败: {}", url))?
                .error_for_status()
                .context(format!("图片请求失败: {}", url))?;

            data = http_resp.bytes().await.context(format!("获取图片数据失败: {}", url))?;

            // 为空说明 JM 缓存失效，带时间戳重试
            if !data.is_empty() {
                break;
            }
        }

        if data.is_empty() {
            anyhow::bail!("图片数据为空: {url}");
        }

        Ok(data.to_vec())
    }

    /// AES-256-ECB 解密（Base64 输入，PKCS7 填充）
    fn decrypt_data(&self, key: &str, data: &str) -> anyhow::Result<String> {
        // 使用Base64解码传入的数据，得到AES-256-ECB加密的数据
        let mut buffer = general_purpose::STANDARD.decode(data).context("Base64 解码失败")?;

        // 验证密钥与数据长度（AES-256 要求 32 字节密钥、16 字节对齐的密文）
        let key: &[u8; 32] = key.as_bytes().try_into().context("密钥长度必须为32字节")?;

        if buffer.is_empty() || buffer.len() % 16 != 0 {
            anyhow::bail!("无效的加密数据长度");
        }

        // 使用 AES-256-ECB（PKCS7 填充）就地解密
        let cipher: Decryptor<Aes256> = Decryptor::new(key.into());
        let decrypted = cipher
            .decrypt_padded::<Pkcs7>(&mut buffer)
            .context("AES-256-ECB 解密失败")?;

        Ok(String::from_utf8(decrypted.to_vec()).context("解密结果不是有效的 UTF-8")?)
    }

    /// 计算图片分块数：未打乱返回 0，打乱则返回分块数（10 或 8）
    fn calculate_block_num(&self, scramble_id: i32, id: i32, filename: &str) -> u32 {
        // 去掉图片扩展名（.webp / .jpg 等），参与 md5 的只有纯图片名
        let filename = filename.rsplit_once('.').map(|(name, _)| name).unwrap_or(filename);

        if id < scramble_id {
            0
        } else if id < 268_850 {
            10
        } else {
            let x = if id < 421_926 { 10 } else { 8 };
            let s = format!("{:x}", md5::compute(format!("{id}{filename}")));
            let last = s.chars().last().and_then(|c| c.to_digit(16)).unwrap_or(0);
            let block_num = (last % x) * 2 + 2;
            block_num
        }
    }

    /// 拼接图片：将被打乱分块的图按块还原为原图
    fn stitch_img(&self, src_img: &image::RgbImage, block_num: u32) -> image::RgbImage {
        let (width, height) = src_img.dimensions();
        // 创建一张空的图片，尺寸与原图相同，用于拼接分块
        let mut stitched_img = image::ImageBuffer::new(width, height);
        // 计算原图像的高度除以 num 的余数
        let remainder_height = height % block_num;
        // 将图片切分为 block_num 块并拼接
        for i in 0..block_num {
            // 计算当前块的标准高度
            let mut block_height = height / block_num;
            // 计算源图像中当前块的 Y 轴起点位置
            let src_img_y_start = height - (block_height * (i + 1)) - remainder_height;
            // 计算目标图像中当前块的 Y 轴起点位置
            let mut dst_img_y_start = block_height * i;
            // 第一块需要加上余数高度，以确保拼接完整
            if i == 0 {
                block_height += remainder_height;
            } else {
                dst_img_y_start += remainder_height;
            }
            // 逐行复制当前块
            for y in 0..block_height {
                let src_y = src_img_y_start + y;
                let dst_y = dst_img_y_start + y;
                // 复制整行像素到目标图像
                for x in 0..width {
                    stitched_img.put_pixel(x, dst_y, *src_img.get_pixel(x, src_y));
                }
            }
        }

        stitched_img
    }
}

// ---------- 客户端构建 ----------

/// 构建 API 客户端（共享 cookie、代理、自动重试）
fn build_api_client(config: &Config, jar: Arc<Jar>) -> ClientWithMiddleware {
    let builder = reqwest::ClientBuilder::new()
        .cookie_provider(jar)
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(5));
    let builder = apply_proxy(config, builder);

    // 重试总时长 5 秒，间隔 1 秒左右（带抖动）
    let retry_policy = ExponentialBackoff::builder()
        .base(1)
        .jitter(Jitter::Bounded)
        .build_with_total_retry_duration(Duration::from_secs(5));

    reqwest_middleware::ClientBuilder::new(builder.build().unwrap())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

/// 构建图片客户端（代理、自动重试，最多 2 次）
fn build_img_client(config: &Config) -> ClientWithMiddleware {
    let builder = reqwest::ClientBuilder::new()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(5));
    let builder = apply_proxy(config, builder);

    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(2);

    reqwest_middleware::ClientBuilder::new(builder.build().unwrap())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

/// 按代理模式应用代理；设置失败时仅记录日志并退回不使用代理
fn apply_proxy(config: &Config, builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    match config.proxy_mode {
        ProxyMode::System => builder,
        ProxyMode::NoProxy => builder.no_proxy(),
        ProxyMode::Custom => {
            let proxy_url = format!("http://{}:{}", config.proxy_host, config.proxy_port);
            match reqwest::Proxy::all(&proxy_url) {
                Ok(proxy) => builder.proxy(proxy),
                Err(err) => {
                    tracing::error!("设置代理 {proxy_url} 失败: {err:#}");
                    builder
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试请求客户端（默认配置）
    fn test_client() -> JMClient {
        JMClient::new(&Config::default())
    }

    /// 打印接口测试结果：成功打印 Debug，失败打印错误（不 panic，便于逐个接口调试）
    fn print_result<T: std::fmt::Debug>(label: &str, result: anyhow::Result<T>) {
        match result {
            Ok(resp) => println!("[{label}] 成功:\n{resp:#?}"),
            Err(e) => println!("[{label}] 失败: {e:#}"),
        }
    }

    #[tokio::test]
    async fn test_request_login() {
        let client = test_client();
        print_result("request_login", client.request_login("", "").await);
    }

    /// 收藏列表依赖登录态，先登录
    #[tokio::test]
    async fn test_request_favorite() {
        let client = test_client();
        client.request_login("likun7402", "likun-7402").await.expect("登录失败");
        print_result(
            "request_favorite",
            client.request_favorite(0, 1, FavoriteSort::UpdateTime).await,
        );
    }

    /// 切换收藏依赖登录态，先登录；调用两次（第二次恢复原状态，避免留下副作用）
    #[tokio::test]
    async fn test_request_toggle_favorite() {
        let client = test_client();
        client.request_login("likun7402", "likun-7402").await.expect("登录失败");
        let album_id = 1444613; // 实际存在的专辑 ID
        print_result(
            "request_toggle_favorite(第1次)",
            client.request_toggle_favorite(album_id).await,
        );
        print_result(
            "request_toggle_favorite(第2次)",
            client.request_toggle_favorite(album_id).await,
        );
    }

    #[tokio::test]
    async fn test_request_album() {
        let client = test_client();
        let album_id = 1444613; // 实际存在的专辑 ID
        print_result("request_album", client.request_album(album_id).await);
    }

    #[tokio::test]
    async fn test_request_chapter() {
        let client = test_client();
        let chapter_id = 1457197; // 实际存在的章节 ID
        print_result("request_chapter", client.request_chapter(chapter_id).await);
    }

    #[tokio::test]
    async fn test_request_promote() {
        let client = test_client();
        match client.request_promote().await {
            Ok(sections) => {
                println!("[request_promote] 成功: 分组数={}", sections.len());
                for section in sections.iter() {
                    println!(
                        "  - Promoteid={:?}, title={}, slug={}, type={:?}, filter_val={:?}, content={}",
                        section.id,
                        section.title,
                        section.slug,
                        section.section_type,
                        section.filter_val,
                        section.content.len()
                    );
                }
            }
            Err(e) => println!("[request_promote] 失败: {e:#}"),
        }
    }

    /// 推荐分组下的分页列表：用「連載更新」分组 id=29 验证
    #[tokio::test]
    async fn test_request_promote_list() {
        let client = test_client();
        print_result("request_promote_list(29)", client.request_promote_list(29, 0).await);
    }

    /// 连载（serialization）接口：type 为 all/manga/hanman，date 为 0~7
    #[tokio::test]
    async fn test_request_serialization() {
        let client = test_client();
        match client.request_serialization("1", "all", 1).await {
            Ok(resp) => {
                println!("[request_serialization] 成功: 条目数={}", resp.list.len());
                for item in resp.list.iter().take(3) {
                    println!(
                        "  - id={}, author={}, name={}, category={:?}, update_at={}",
                        item.id, item.author, item.name, item.category, item.update_at
                    );
                }
            }
            Err(e) => println!("[request_serialization] 失败: {e:#}"),
        }
    }

    /// 搜索两次：一次按 JM 号搜索（返回 redirect_aid），一次按关键字搜索（返回结果列表）
    #[tokio::test]
    async fn test_request_search() {
        let client = test_client();
        // 按 JM 号搜索：直接打印完整 Debug
        print_result(
            "request_search(1444613)",
            client.request_search("1444613", 1, SearchSort::Latest).await,
        );
        // 按关键字搜索：只打印概要，避免 content 过长刷屏
        match client.request_search("原神", 1, SearchSort::Latest).await {
            Ok(resp) => {
                println!(
                    "[request_search(原神)] 成功: search_query={}, total={}, content={}, redirect_aid={:?}",
                    resp.search_query,
                    resp.total,
                    resp.content.len(),
                    resp.redirect_aid
                );
                for item in resp.content.iter().take(3) {
                    println!("  - id={}, author={}, name={}", item.id, item.author, item.name);
                }
            }
            Err(e) => println!("[request_search(原神)] 失败: {e:#}"),
        }
    }

    /// 分类列表：打印全部分类概要
    #[tokio::test]
    async fn test_request_categories() {
        let client = test_client();
        match client.request_categories().await {
            Ok(resp) => println!("CategoryResp = {:#?}", resp),
            Err(e) => println!("[request_categories] 失败: {e:#}"),
        }
    }

    /// 分类下的专辑列表：用空 slug（最新A漫）验证
    #[tokio::test]
    async fn test_request_categories_filter() {
        let client = test_client();
        match client.request_categories_filter("", 0, CategorySort::Latest).await {
            Ok(resp) => {
                println!(
                    "[request_categories_filter(slug=空)] 成功: search_query={}, total={}, content={}, redirect_aid={:?}",
                    resp.search_query,
                    resp.total,
                    resp.content.len(),
                    resp.redirect_aid
                );
                for item in resp.content.iter().take(3) {
                    println!("  - id={}, author={}, name={}", item.id, item.author, item.name);
                }
            }
            Err(e) => println!("[request_categories_filter(slug=空)] 失败: {e:#}"),
        }
    }

    #[tokio::test]
    async fn test_request_scramble_id() {
        let client = test_client();
        print_result("request_scramble_id", client.request_scramble_id(1444613).await);
    }

    /// 封面只打印字节数（不打印二进制内容）；image_name 可为 {album_id}.jpg 或 {album_id}_3x4.jpg
    #[tokio::test]
    async fn test_request_cover() {
        let client = test_client();
        let image_name = "1444613_3x4.jpg"; // 实际存在的专辑封面文件名
        match client.request_cover(image_name).await {
            Ok(bytes) => println!("[request_cover] 成功: 字节数={}", bytes.len()),
            Err(e) => println!("[request_cover] 失败: {e:#}"),
        }
    }

    /// 章节图片只打印字节数（不打印二进制内容）；先获取 scramble_id 再取图（解密还原分块）
    #[tokio::test]
    async fn test_request_photo() {
        let client = test_client();
        let scramble_id = APP_SCRAMBLE_ID;
        let chapter_id = 1444613; // 实际存在的章节 ID
        let image_name = "00001.webp"; // 章节返回的图片名

        match client.request_photo(scramble_id, chapter_id, image_name).await {
            Ok(bytes) => {
                println!("[request_photo] 成功: 字节数={}", bytes.len());
                // 写入系统临时目录便于直接查看图片内容
                let path = std::env::temp_dir().join("koma_photo_test.webp");
                std::fs::write(&path, &bytes).expect("写入图片文件失败");
                println!("[request_photo] 已保存到 {}", path.display());
            }
            Err(e) => println!("[request_photo] 失败: {e:#}"),
        }
    }
}
