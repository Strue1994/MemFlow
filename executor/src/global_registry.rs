use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalPattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub workflow_json: String,
    pub source_repo: String,
    pub source_url: String,
    pub stars: u32,
    pub downloaded_count: u32,
    pub node_types: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub local_synced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub name: String,
    pub url: String,
    pub branch: String,
    pub workflow_path: String,
    pub auth_token: Option<String>,
    pub sync_interval_hours: u32,
}

impl Default for RepoConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            url: String::new(),
            branch: "main".to_string(),
            workflow_path: "workflows".to_string(),
            auth_token: None,
            sync_interval_hours: 24,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalRegistryConfig {
    pub local_cache_path: String,
    pub sync_on_startup: bool,
    pub repos: Vec<RepoConfig>,
}

impl Default for GlobalRegistryConfig {
    fn default() -> Self {
        Self {
            local_cache_path: "./global_patterns".to_string(),
            sync_on_startup: true,
            repos: Vec::new(),
        }
    }
}

pub struct GlobalRegistry {
    config: GlobalRegistryConfig,
    patterns: Arc<RwLock<Vec<GlobalPattern>>>,
    last_sync: Arc<RwLock<Option<i64>>>,
    sync_in_progress: Arc<RwLock<bool>>,
}

impl GlobalRegistry {
    pub fn new(config: GlobalRegistryConfig) -> Self {
        Self {
            config,
            patterns: Arc::new(RwLock::new(Vec::new())),
            last_sync: Arc::new(RwLock::new(None)),
            sync_in_progress: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn add_repo(&mut self, repo: RepoConfig) {
        self.config.repos.push(repo);
    }

    pub async fn sync_all(&self) -> anyhow::Result<SyncResult> {
        let mut in_progress = self.sync_in_progress.write().await;
        if *in_progress {
            return Ok(SyncResult {
                added: 0,
                updated: 0,
                removed: 0,
                errors: vec!["Sync already in progress".to_string()],
            });
        }
        *in_progress = true;
        drop(in_progress);

        let mut result = SyncResult::default();
        let mut all_patterns = Vec::new();

        for repo in &self.config.repos {
            match self.sync_repo(repo).await {
                Ok(patterns) => {
                    all_patterns.extend(patterns);
                }
                Err(e) => {
                    result.errors.push(format!("{}: {}", repo.name, e));
                }
            }
        }

        let mut local = self.patterns.write().await;
        let old_count = local.len();
        *local = all_patterns;
        result.added = local.len();
        result.updated = local.len().saturating_sub(old_count);

        *self.last_sync.write().await = Some(chrono::Utc::now().timestamp());

        let mut in_progress = self.sync_in_progress.write().await;
        *in_progress = false;

        Ok(result)
    }

    async fn sync_repo(&self, repo: &RepoConfig) -> anyhow::Result<Vec<GlobalPattern>> {
        let client = reqwest::Client::new();
        
        let url = format!("{}/contents/{}?ref={}", repo.url, repo.workflow_path, repo.branch);
        
        let mut request = client.get(&url);
        if let Some(token) = &repo.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch repo: {}", response.status()));
        }

        let contents: Vec<RepoContent> = response.json().await?;
        let mut patterns = Vec::new();

        for item in contents.iter().filter(|c| c.name.ends_with(".json") || c.name.ends_with(".yaml")) {
            let pattern = self.fetch_workflow_file(repo, &item.name).await?;
            if let Some(p) = pattern {
                patterns.push(p);
            }
        }

        Ok(patterns)
    }

    async fn fetch_workflow_file(&self, repo: &RepoConfig, filename: &str) -> anyhow::Result<Option<GlobalPattern>> {
        let client = reqwest::Client::new();
        let url = format!("{}/contents/{}/{}", repo.url, repo.workflow_path, filename);
        
        let mut request = client.get(&url);
        if let Some(token) = &repo.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Ok(None);
        }

        let content: RepoContent = response.json().await?;
        
        let decoded = if let Some(encoded) = content.content {
            let decoded = base64_decode(&encoded);
            String::from_utf8(decoded).unwrap_or_default()
        } else {
            return Ok(None);
        };

        let node_types = self.extract_node_types(&decoded);
        let (name, description, tags) = self.extract_metadata(&decoded, filename);

        let pattern = GlobalPattern {
            id: format!("{}_{}", repo.name, filename),
            name,
            description,
            tags,
            workflow_json: decoded,
            source_repo: repo.name.clone(),
            source_url: url,
            stars: 0,
            downloaded_count: 0,
            node_types,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            local_synced: true,
        };

        Ok(Some(pattern))
    }

    fn extract_node_types(&self, json: &str) -> Vec<String> {
        let mut types = Vec::new();
        
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json) {
            if let Some(nodes) = value.get("nodes").and_then(|n| n.as_array()) {
                for node in nodes {
                    if let Some(ntype) = node.get("type").and_then(|t| t.as_str()) {
                        if !types.contains(&ntype.to_string()) {
                            types.push(ntype.to_string());
                        }
                    }
                }
            }
        }
        
        types
    }

    fn extract_metadata(&self, json: &str, filename: &str) -> (String, String, Vec<String>) {
        let name = filename.trim_end_matches(".json").trim_end_matches(".yaml")
            .replace("_", " ")
            .replace("-", " ");

        let description = String::new();
        let tags = vec!["community".to_string()];

        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json) {
            if let Some(n) = value.get("name").and_then(|n| n.as_str()) {
                return (n.to_string(), description, tags);
            }
        }

        (name, description, tags)
    }

    pub async fn search(&self, query: &str, tags: Option<&[String]>, limit: usize) -> Vec<GlobalPattern> {
        let patterns = self.patterns.read().await;
        let query_lower = query.to_lowercase();
        
        let mut results: Vec<_> = patterns.iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower) ||
                p.description.to_lowercase().contains(&query_lower) ||
                p.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .filter(|p| {
                if let Some(t) = tags {
                    t.iter().any(|tag| p.tags.contains(tag))
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        results.sort_by(|a, b| b.stars.cmp(&a.stars));
        results.truncate(limit);
        
        results
    }

    pub async fn get_by_id(&self, id: &str) -> Option<GlobalPattern> {
        self.patterns.read().await.iter().find(|p| p.id == id).cloned()
    }

    pub async fn get_by_node_type(&self, node_type: &str) -> Vec<GlobalPattern> {
        self.patterns.read().await.iter()
            .filter(|p| p.node_types.contains(&node_type.to_string()))
            .cloned()
            .collect()
    }

    pub async fn increment_download(&self, id: &str) -> anyhow::Result<()> {
        let mut patterns = self.patterns.write().await;
        if let Some(pattern) = patterns.iter_mut().find(|p| p.id == id) {
            pattern.downloaded_count += 1;
        }
        Ok(())
    }

    pub async fn get_all(&self) -> Vec<GlobalPattern> {
        self.patterns.read().await.clone()
    }

    pub async fn get_last_sync(&self) -> Option<i64> {
        *self.last_sync.read().await
    }

    pub async fn is_syncing(&self) -> bool {
        *self.sync_in_progress.read().await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RepoContent {
    name: String,
    path: String,
    content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncResult {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
    pub errors: Vec<String>,
}

fn base64_decode(input: &str) -> Vec<u8> {
    let cleaned = input.replace('\n', "");
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &cleaned);
    decoded.unwrap_or_default()
}

pub fn create_global_registry(config: GlobalRegistryConfig) -> GlobalRegistry {
    GlobalRegistry::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_creation() {
        let config = GlobalRegistryConfig::default();
        let registry = GlobalRegistry::new(config);
        let patterns = registry.get_all().await;
        assert!(patterns.is_empty());
    }

    #[tokio::test]
    async fn test_search() {
        let config = GlobalRegistryConfig::default();
        let registry = GlobalRegistry::new(config);
        let results = registry.search("test", None, 10).await;
        assert!(results.is_empty());
    }
}