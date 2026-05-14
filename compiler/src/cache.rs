use crate::error::ParseError;
use crate::ir::{Workflow, WorkflowNode};
use crate::parser::parse_n8n_workflow;
use redis::{Client, Commands};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;

const CACHE_PREFIX: &str = "workflow:compiled:";
const DEFAULT_TTL_SECONDS: u64 = 7 * 24 * 3600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedWorkflow {
    pub workflow: Workflow,
    pub compiled_at: i64,
    pub json_hash: String,
}

pub struct WorkflowCache {
    client: Option<Arc<Client>>,
    enabled: bool,
    ttl_seconds: u64,
    local_cache: Arc<RwLock<lru::LruCache<String, CachedWorkflow>>>,
}

impl WorkflowCache {
    pub fn new(redis_url: Option<&str>, ttl_seconds: u64) -> Self {
        let client = redis_url.and_then(|url| Client::open(url).ok()).map(Arc::new);
        
        Self {
            client,
            enabled: client.is_some(),
            ttl_seconds: if ttl_seconds > 0 { ttl_seconds } else { DEFAULT_TTL_SECONDS },
            local_cache: Arc::new(RwLock::new(lru::LruCache::new(100))),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn compute_key(&self, json: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        let result = hasher.finalize();
        format!("{}{:x}", CACHE_PREFIX, result)
    }

    pub async fn get(&self, json: &str) -> Option<Workflow> {
        let key = self.compute_key(json);
        
        {
            let cache = self.local_cache.read().await;
            if let Some(cached) = cache.get(&key) {
                return Some(cached.workflow.clone());
            }
        }
        
        if let Some(client) = &self.client {
            let mut con = client.get_connection().ok()?;
            let cached: Option<CachedWorkflow> = con.get(&key).ok()?;
            
            if let Some(cached) = cached {
                let mut cache = self.local_cache.write().await;
                cache.put(key.clone(), cached.clone());
                return Some(cached.workflow);
            }
        }
        
        None
    }

    pub async fn set(&self, json: &str, workflow: Workflow) -> Result<(), ParseError> {
        let key = self.compute_key(json);
        let cached = CachedWorkflow {
            workflow: workflow.clone(),
            compiled_at: chrono::Utc::now().timestamp(),
            json_hash: key.clone(),
        };
        
        {
            let mut cache = self.local_cache.write().await;
            cache.put(key.clone(), cached.clone());
        }
        
        if let Some(client) = &self.client {
            if let Ok(mut con) = client.get_connection() {
                let _: () = con.set_ex(&key, &cached, self.ttl_seconds as usize).ok();
            }
        }
        
        Ok(())
    }

    pub async fn invalidate(&self, json: &str) {
        let key = self.compute_key(json);
        
        {
            let mut cache = self.local_cache.write().await;
            cache.remove(&key);
        }
        
        if let Some(client) = &self.client {
            let _: () = client.get_connection()
                .and_then(|mut con| con.del(&key))
                .ok();
        }
    }

    pub async fn clear(&self) {
        {
            let mut cache = self.local_cache.write().await;
            cache.clear();
        }
        
        if let Some(client) = &self.client {
            let pattern = format!("{}*", CACHE_PREFIX);
            let _: () = client.get_connection()
                .and_then(|mut con| con.keys(&pattern))
                .and_then(|keys| {
                    for key in keys {
                        let _: () = con.del(key)?;
                    }
                    Ok(())
                })
                .ok();
        }
    }

    pub async fn get_stats(&self) -> CacheStats {
        let local_size = self.local_cache.read().await.len();
        
        let remote_size = if let Some(client) = &self.client {
            client.get_connection()
                .and_then(|mut con| {
                    let pattern = format!("{}*", CACHE_PREFIX);
                    let keys: Vec<String> = con.keys(&pattern)?;
                    Ok(keys.len())
                })
                .unwrap_or(0)
        } else {
            0
        };

        CacheStats {
            enabled: self.enabled,
            local_entries: local_size,
            remote_entries: remote_size,
            ttl_seconds: self.ttl_seconds,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub enabled: bool,
    pub local_entries: usize,
    pub remote_entries: usize,
    pub ttl_seconds: u64,
}

pub async fn parse_with_cache(
    cache: &WorkflowCache,
    json: &str,
) -> Result<Workflow, ParseError> {
    if cache.is_enabled() {
        if let Some(workflow) = cache.get(json).await {
            return Ok(workflow);
        }
    }
    
    let workflow = parse_n8n_workflow(json)?;
    
    if cache.is_enabled() {
        let _ = cache.set(json, workflow.clone()).await;
    }
    
    Ok(workflow)
}

pub fn create_workflow_cache(redis_url: Option<&str>) -> WorkflowCache {
    WorkflowCache::new(redis_url, DEFAULT_TTL_SECONDS)
}

impl Default for WorkflowCache {
    fn default() -> Self {
        Self::new(None, DEFAULT_TTL_SECONDS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_key() {
        let cache = WorkflowCache::default();
        let key1 = cache.compute_key(r#"{"name":"test"}"#);
        let key2 = cache.compute_key(r#"{"name":"test"}"#);
        let key3 = cache.compute_key(r#"{"name":"other"}"#);
        
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[tokio::test]
    async fn test_local_cache() {
        let cache = WorkflowCache::default();
        
        let json = r#"{"name":"test","nodes":[]}"#;
        let workflow = Workflow {
            id: "test".to_string(),
            name: "test".to_string(),
            nodes: vec![],
            connections: std::collections::HashMap::new(),
            settings: Default::default(),
        };
        
        cache.set(json, workflow.clone()).await.unwrap();
        
        let cached = cache.get(json).await;
        assert!(cached.is_some());
    }
}