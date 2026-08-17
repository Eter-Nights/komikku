use crate::comic_source::jmcomic::client::{ApiDomain, ImageDomain};

use serde::{Deserialize, Serialize};

/// 代理模式
#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProxyMode {
    /// 使用系统代理
    #[default]
    System,
    /// 不使用代理
    NoProxy,
    /// 使用自定义代理
    Custom,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub username: String,
    pub password: String,
    pub api_domain: ApiDomain,
    pub image_domain: ImageDomain,
    pub proxy_mode: ProxyMode,
    pub proxy_host: String,
    pub proxy_port: u16,
    /// 图片下载并发数
    pub img_concurrency: u32,
    /// 图片缓存清理天数
    pub cache_cleanup_days: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            username: String::new(),
            password: String::new(),
            api_domain: ApiDomain::default(),
            image_domain: ImageDomain::default(),
            proxy_mode: ProxyMode::default(),
            proxy_host: "127.0.0.1".to_string(),
            proxy_port: 7890,
            img_concurrency: 10,
            cache_cleanup_days: 7,
        }
    }
}

impl Config {
    /// 从指定目录加载配置（读取 `config.json`）；文件不存在时用默认配置并写回磁盘
    pub async fn load_from_dir(dir: &std::path::Path) -> anyhow::Result<Self> {
        let path = dir.join("config.json");
        let config = match tokio::fs::read_to_string(&path).await {
            Ok(json) => serde_json::from_str::<Config>(&json)?,
            Err(_) => {
                let config = Config::default();
                config.save_to_dir(dir).await?;
                config
            }
        };
        Ok(config)
    }

    /// 判断新配置是否需要重建内层资源（域名/代理变化重建客户端，并发数变化重建信号量）
    pub fn needs_reload(&self, new: &Config) -> bool {
        self.api_domain != new.api_domain
            || self.image_domain != new.image_domain
            || self.proxy_mode != new.proxy_mode
            || self.proxy_host != new.proxy_host
            || self.proxy_port != new.proxy_port
            || self.img_concurrency != new.img_concurrency
    }

    /// 将配置写入指定目录下的 `config.json`
    pub async fn save_to_dir(&self, dir: &std::path::Path) -> anyhow::Result<()> {
        let path = dir.join("config.json");
        let json = serde_json::to_string_pretty(self)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }
}
