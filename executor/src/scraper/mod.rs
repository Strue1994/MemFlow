pub mod fusion;
pub mod upgrade;
pub mod security;
pub mod knowledge;

pub use knowledge::KnowledgeEngine;

use serde::{Deserialize, Serialize};
use reqwest::Client;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SourceType {
    Http,
    Rss,
    GitHub,
    Sitemap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSource {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(rename = "type")]
    pub source_type: SourceType,
    pub enabled: bool,
    pub schedule: Schedule,
    pub auth: Option<AuthConfig>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub interval_secs: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub auth_type: String,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperConfig {
    pub sources: Vec<KnowledgeSource>,
    #[serde(skip)]
    client: Client,
    pub rate_limit_ms: u64,
}

impl ScraperConfig {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            rate_limit_ms: 1000,
        }
    }

    pub fn add_source(&mut self, source: KnowledgeSource) {
        self.sources.push(source);
    }

    pub fn get_enabled_sources(&self) -> Vec<&KnowledgeSource> {
        self.sources.iter().filter(|s| s.enabled).collect()
    }
}

impl Default for ScraperConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedContent {
    pub source_id: String,
    pub url: String,
    pub content: String,
    pub fetched_at: String,
    pub content_type: String,
}

pub struct WebScraper {
    config: ScraperConfig,
}

impl WebScraper {
    pub fn new(config: ScraperConfig) -> Self {
        Self { config }
    }

    pub async fn fetch(&self, source: &KnowledgeSource) -> Result<ScrapedContent> {
        let client = &self.config.client;
        
        let mut request = client.get(&source.url);
        
        if let Some(ref auth) = source.auth {
            if let Some(ref token) = auth.token {
                request = request.header("Authorization", format!("Bearer {}", token));
            }
        }

        let response = request.send().await?;
        let body = response.text().await?;

        Ok(ScrapedContent {
            source_id: source.id.clone(),
            url: source.url.clone(),
            content: body,
            fetched_at: chrono::Utc::now().to_rfc3339(),
            content_type: "text/html".to_string(),
        })
    }

    pub async fn fetch_all(&self) -> Vec<Result<ScrapedContent>> {
        let sources = self.config.get_enabled_sources();
        let mut results = Vec::new();

        for source in sources {
            let result = self.fetch(source).await;
            results.push(result);
            tokio::time::sleep(tokio::time::Duration::from_millis(self.config.rate_limit_ms)).await;
        }

        results
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssItem {
    pub title: String,
    pub link: String,
    pub description: String,
    pub pub_date: String,
}

pub struct RssScraper {
    client: Client,
}

impl RssScraper {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn fetch_feed(&self, url: &str) -> Result<Vec<RssItem>> {
        let response = self.client.get(url).send().await?;
        let _body = response.text().await?;
        
        Ok(Vec::new())
    }
}