use tokio::sync::RwLock;

use crate::scraper::{fusion::{Conflict, KnowledgeFusion, StaleDetector}, KnowledgeSource, WebScraper};

pub struct KnowledgeEngine {
    sources: RwLock<Vec<KnowledgeSource>>,
    fusion: KnowledgeFusion,
    stale_detector: StaleDetector,
    scraper: Option<WebScraper>,
}

impl KnowledgeEngine {
    pub fn new() -> Self {
        Self {
            sources: RwLock::new(Vec::new()),
            fusion: KnowledgeFusion::new(0.8),
            stale_detector: StaleDetector::new(30),
            scraper: None,
        }
    }

    pub async fn add_source(&self, source: KnowledgeSource) {
        let mut sources = self.sources.write().await;
        sources.push(source);
    }

    pub async fn list_sources(&self) -> Vec<KnowledgeSource> {
        let sources = self.sources.read().await;
        sources.clone()
    }

    pub async fn get_stale_knowledge(&self) -> Vec<String> {
        self.stale_detector.detect(&[]);
        Vec::new()
    }

    pub async fn detect_conflicts(&self) -> Vec<Conflict> {
        Vec::new()
    }
}

impl Default for KnowledgeEngine {
    fn default() -> Self {
        Self::new()
    }
}