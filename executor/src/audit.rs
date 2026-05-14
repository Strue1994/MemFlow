use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<String>,
    pub action: String,
    pub resource: String,
    pub resource_id: String,
    pub result: AuditResult,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditResult {
    Success,
    Failure,
    Denied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFilter {
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub resource: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

pub struct AuditLogger {
    logs: Arc<RwLock<VecDeque<AuditLog>>>,
    max_size: usize,
}

impl AuditLogger {
    pub fn new(max_size: usize) -> Self {
        Self {
            logs: Arc::new(RwLock::new(VecDeque::with_capacity(max_size))),
            max_size,
        }
    }

    pub async fn log(&self, log: AuditLog) {
        let mut logs = self.logs.write().await;
        if logs.len() >= self.max_size {
            logs.pop_front();
        }
        logs.push_back(log);
    }

    pub async fn query(&self, filter: AuditFilter) -> Vec<AuditLog> {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|log| {
                if let Some(ref uid) = filter.user_id {
                    if log.user_id.as_ref() != Some(uid) {
                        return false;
                    }
                }
                if let Some(ref action) = filter.action {
                    if &log.action != action {
                        return false;
                    }
                }
                if let Some(ref resource) = filter.resource {
                    if &log.resource != resource {
                        return false;
                    }
                }
                if let Some(from) = filter.from {
                    if log.timestamp < from {
                        return false;
                    }
                }
                if let Some(to) = filter.to {
                    if log.timestamp > to {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    pub async fn get_logs(&self, limit: usize) -> Vec<AuditLog> {
        let logs = self.logs.read().await;
        logs.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[allow(dead_code)]
pub fn create_audit_log(
    user_id: Option<String>,
    action: &str,
    resource: &str,
    resource_id: &str,
    result: AuditResult,
    details: Option<serde_json::Value>,
    ip_address: Option<String>,
) -> AuditLog {
    AuditLog {
        id: format!("audit_{}", uuid::Uuid::new_v4()),
        timestamp: Utc::now(),
        user_id,
        action: action.to_string(),
        resource: resource.to_string(),
        resource_id: resource_id.to_string(),
        result,
        details,
        ip_address,
    }
}