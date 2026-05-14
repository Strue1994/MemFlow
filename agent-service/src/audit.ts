use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub user_id: String,
    pub action: AuditAction,
    pub resource_type: String,
    pub resource_id: String,
    pub details: HashMap<String, serde_json::Value>,
    pub ip_address: String,
    pub success: bool,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    WorkflowCreate,
    WorkflowUpdate,
    WorkflowDelete,
    WorkflowExecute,
    WorkflowRollback,
    ApiKeyCreate,
    ApiKeyRevoke,
    SettingsUpdate,
    UserRoleChange,
}

impl AuditEvent {
    pub fn new(
        user_id: &str,
        action: AuditAction,
        resource_type: &str,
        resource_id: &str,
        ip: &str,
    ) -> Self {
        Self {
            id: format!("audit_{}", uuid::Uuid::new_v4()),
            user_id: user_id.to_string(),
            action,
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            details: HashMap::new(),
            ip_address: ip.to_string(),
            success: true,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn set_details(&mut self, details: HashMap<String, serde_json::Value>) {
        self.details = details;
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

pub struct AuditLogger {
    events: std::sync::Mutex<Vec<AuditEvent>>,
    max_events: usize,
}

impl AuditLogger {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
            max_events,
        }
    }

    pub fn log(&self, event: AuditEvent) {
        let mut events = self.events.lock().unwrap();
        events.push(event);
        if events.len() > self.max_events {
            events.drain(0..1000);
        }
    }

    pub fn query(&self, user_id: Option<&str>, action: Option<&str>, limit: usize) -> Vec<AuditEvent> {
        let events = self.events.lock().unwrap();
        events
            .iter()
            .rev()
            .filter(|e| {
                if let Some(uid) = user_id {
                    if &e.user_id != uid {
                        return false;
                    }
                }
                if let Some(act) = action {
                    if !format!("{:?}", e.action).contains(act) {
                        return false;
                    }
                }
                true
            })
            .take(limit)
            .cloned()
            .collect()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(10000)
    }
}