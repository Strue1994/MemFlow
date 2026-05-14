/// P3.4: Webhook target registry
/// Stores webhook → workflow mappings. HTTP listener runs in gateway.

use std::collections::HashMap;
use std::sync::Mutex;

static WEBHOOK_TARGETS: once_cell::sync::Lazy<Mutex<HashMap<String, String>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

pub fn register_webhook(path: &str, workflow_id: &str) {
    WEBHOOK_TARGETS.lock().unwrap().insert(path.to_string(), workflow_id.to_string());
    tracing::info!(target: "executor.webhook", path = %path, workflow = %workflow_id, "Webhook registered");
}

pub fn get_webhook_target(path: &str) -> Option<String> {
    WEBHOOK_TARGETS.lock().unwrap().get(path).cloned()
}

pub fn list_webhooks() -> Vec<String> {
    WEBHOOK_TARGETS.lock().unwrap().keys().cloned().collect()
}

pub fn unregister_webhook(path: &str) {
    WEBHOOK_TARGETS.lock().unwrap().remove(path);
}
