/// T4.4: Rust-native scheduler (replacing Python scheduler/)
/// Provides cron-based and interval-based task scheduling

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleMode { Cron(String), Interval { seconds: u64 }, Once }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub workflow_id: String,
    pub mode: ScheduleMode,
    pub next_run: Option<String>,
    pub last_run: Option<String>,
    pub enabled: bool,
    pub params: serde_json::Value,
}

static SCHEDULER: once_cell::sync::Lazy<Mutex<HashMap<String, ScheduledTask>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

pub fn schedule_task(workflow_id: &str, mode: ScheduleMode, params: serde_json::Value) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let task = ScheduledTask {
        id: id.clone(), workflow_id: workflow_id.to_string(), mode,
        next_run: Some(now.clone()), last_run: None, enabled: true, params,
    };
    SCHEDULER.lock().unwrap().insert(id.clone(), task);
    id
}

pub fn list_scheduled() -> Vec<ScheduledTask> {
    SCHEDULER.lock().unwrap().values().cloned().collect()
}

pub fn cancel_task(id: &str) -> bool {
    SCHEDULER.lock().unwrap().remove(id).is_some()
}

pub fn scheduled_count() -> usize {
    SCHEDULER.lock().unwrap().len()
}
