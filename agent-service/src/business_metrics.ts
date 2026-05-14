use std::sync::Arc;
use tokio::sync::RwLock;

pub struct BusinessMetrics {
    workflow_creations: Arc<RwLock<WorkflowCreationCounter>>,
    active_users: Arc<RwLock<ActiveUserCounter>>,
    tokens_saved: Arc<RwLock<TokenSavingsCounter>>,
}

#[derive(Default)]
pub struct WorkflowCreationCounter {
    by_natural_language: u64,
    by_template: u64,
    by_import: u64,
    total: u64,
}

#[derive(Default)]
pub struct ActiveUserCounter {
    daily_users: std::collections::HashSet<String>,
    weekly_users: std::collections::HashSet<String>,
}

#[derive(Default)]
pub struct TokenSavingsCounter {
    total_saved: u64,
    cache_hits: u64,
    total_requests: u64,
}

impl BusinessMetrics {
    pub fn new() -> Self {
        Self {
            workflow_creations: Arc::new(RwLock::new(WorkflowCreationCounter::default())),
            active_users: Arc::new(RwLock::new(ActiveUserCounter::default())),
            tokens_saved: Arc::new(RwLock::new(TokenSavingsCounter::default())),
        }
    }

    pub async fn record_workflow_creation(&self, method: &str) {
        let mut counter = self.workflow_creations.write().await;
        counter.total += 1;
        match method {
            "natural_language" => counter.by_natural_language += 1,
            "template" => counter.by_template += 1,
            "import" => counter.by_import += 1,
            _ => {}
        }
    }

    pub async fn record_user_activity(&self, user_id: &str) {
        let mut users = self.active_users.write().await;
        users.daily_users.insert(user_id.to_string());
    }

    pub async fn record_token_savings(&self, saved: u64) {
        let mut savings = self.tokens_saved.write().await;
        savings.total_saved += saved;
        savings.total_requests += 1;
    }

    pub async fn get_metrics(&self) -> MetricsSnapshot {
        let creations = self.workflow_creations.read().await;
        let savings = self.tokens_saved.read().await;
        let users = self.active_users.read().await;

        let cache_hit_rate = if savings.total_requests > 0 {
            savings.cache_hits as f64 / savings.total_requests as f64
        } else {
            0.0
        };

        MetricsSnapshot {
            workflow_creations_total: creations.total,
            workflow_by_natural_language: creations.by_natural_language,
            workflow_by_template: creations.by_template,
            workflow_by_import: creations.by_import,
            active_users_daily: users.daily_users.len() as u64,
            tokens_saved_total: savings.total_saved,
            cache_hit_rate,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    pub workflow_creations_total: u64,
    pub workflow_by_natural_language: u64,
    pub workflow_by_template: u64,
    pub workflow_by_import: u64,
    pub active_users_daily: u64,
    pub tokens_saved_total: u64,
    pub cache_hit_rate: f64,
}

pub fn create_business_metrics() -> BusinessMetrics {
    BusinessMetrics::new()
}