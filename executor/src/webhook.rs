use crate::error::ExecError;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

static WEBHOOKS: once_cell::sync::Lazy<Mutex<HashMap<String, (String, String)>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

pub fn execute_webhook_register(path: &str, method: &str, workflow: &str) -> Result<Value, ExecError> {
    WEBHOOKS.lock().unwrap().insert(path.to_string(), (method.to_uppercase(), workflow.to_string()));
    Ok(serde_json::json!({"status": "registered", "path": path}))
}

pub fn execute_webhook_dispatch(path: &str) -> Result<Value, ExecError> {
    let hooks = WEBHOOKS.lock().unwrap();
    match hooks.get(path) {
        Some((method, wf)) => Ok(serde_json::json!({"status": "dispatched", "path": path, "method": method, "workflow": wf})),
        None => Err(ExecError::HttpError(format!("No webhook at '{}'", path))),
    }
}
