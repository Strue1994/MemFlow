use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Editor,
    Viewer,
}

impl Default for Role {
    fn default() -> Self {
        Role::Viewer
    }
}

#[derive(Debug, Clone)]
pub struct ApiKey {
    pub key: String,
    pub role: Role,
    pub rate_limit: u32,
    pub created_at: i64,
}

impl ApiKey {
    pub fn can_execute(&self) -> bool {
        matches!(self.role, Role::Admin | Role::Editor | Role::Viewer)
    }

    pub fn can_edit(&self) -> bool {
        matches!(self.role, Role::Admin | Role::Editor)
    }

    pub fn can_admin(&self) -> bool {
        matches!(self.role, Role::Admin)
    }
}

static API_KEYS: Lazy<Arc<RwLock<HashMap<String, ApiKey>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});

static RATE_LIMITER: Lazy<Arc<RwLock<HashMap<String, (u32, Instant)>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});

pub async fn init_api_keys(keys: Vec<(String, Role, u32)>) {
    let mut map = API_KEYS.write().await;
    for (key, role, rate_limit) in keys {
        map.insert(key.clone(), ApiKey {
            key,
            role,
            rate_limit,
            created_at: chrono::Utc::now().timestamp(),
        });
    }
}

pub async fn add_api_key(key: String, role: Role, rate_limit: u32) {
    let mut map = API_KEYS.write().await;
    map.insert(key.clone(), ApiKey {
        key,
        role,
        rate_limit,
        created_at: chrono::Utc::now().timestamp(),
    });
}

pub async fn remove_api_key(key: &str) -> bool {
    let mut map = API_KEYS.write().await;
    map.remove(key).is_some()
}

pub async fn validate_api_key(key: &str) -> Option<ApiKey> {
    let map = API_KEYS.read().await;
    map.get(key).cloned()
}

pub async fn check_rate_limit(key: &str) -> bool {
    let api_key = validate_api_key(key).await;
    if let Some(api_key) = api_key {
        let limit = api_key.rate_limit;
        if limit == 0 {
            return true;
        }

        let mut limiter = RATE_LIMITER.write().await;
        let now = Instant::now();
        
        let should_allow = if let Some((count, start)) = limiter.get(key) {
            if start.elapsed() > Duration::from_secs(60) {
                true
            } else {
                *count < limit
            }
        } else {
            true
        };
        
        if should_allow {
            let new_count = limiter.get(key).map(|(c, _)| *c).unwrap_or(0) + 1;
            limiter.insert(key.to_string(), (new_count, now));
        }
        
        should_allow
    } else {
        false
    }
}

pub async fn list_api_keys() -> Vec<(String, Role, u32)> {
    let map = API_KEYS.read().await;
    map.values().map(|k| (k.key.clone(), k.role, k.rate_limit)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_key_validation() {
        add_api_key("test-key".to_string(), Role::Admin, 100).await;
        let key = validate_api_key("test-key").await;
        assert!(key.is_some());
        assert!(key.unwrap().can_admin());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        add_api_key("rate-key".to_string(), Role::Viewer, 5).await;
        for _ in 0..5 {
            assert!(check_rate_limit("rate-key").await);
        }
    }
}