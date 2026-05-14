use crate::error::ExecError;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

static SCHEDULED: once_cell::sync::Lazy<Mutex<HashMap<String, cron::Schedule>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

pub fn execute_schedule_cron(expression: &str, _workflow_id: &str) -> Result<Value, ExecError> {
    let schedule = expression.parse::<cron::Schedule>()
        .map_err(|e| ExecError::HttpError(format!("Invalid cron '{}': {}", expression, e)))?;
    let mut jobs = SCHEDULED.lock().unwrap();
    let job_id = format!("job_{}", jobs.len() + 1);
    jobs.insert(job_id.clone(), schedule);
    tracing::info!(target: "executor.cron", job_id = %job_id, expr = %expression, "Cron scheduled");
    Ok(serde_json::json!({"status": "scheduled", "job_id": job_id, "expression": expression}))
}

pub fn list_scheduled_jobs() -> Vec<String> {
    SCHEDULED.lock().unwrap().keys().cloned().collect()
}
