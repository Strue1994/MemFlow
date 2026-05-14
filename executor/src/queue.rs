use crate::error::ExecError;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::broadcast;

static QUEUES: once_cell::sync::Lazy<Mutex<HashMap<String, broadcast::Sender<Value>>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

fn get_q(name: &str) -> broadcast::Sender<Value> {
    let mut qs = QUEUES.lock().unwrap();
    qs.get(name).cloned().unwrap_or_else(|| {
        let (tx, _) = broadcast::channel(1000);
        qs.insert(name.to_string(), tx.clone()); tx
    })
}

pub fn execute_queue_publish(name: &str, msg: &Value) -> Result<Value, ExecError> {
    let _ = get_q(name).send(msg.clone());
    Ok(serde_json::json!({"status": "published", "queue": name}))
}

pub fn execute_queue_consume(name: &str) -> Result<Value, ExecError> {
    let mut rx = get_q(name).subscribe();
    match rx.try_recv() {
        Ok(msg) => Ok(serde_json::json!({"status": "consumed", "queue": name, "message": msg})),
        Err(broadcast::error::TryRecvError::Empty) => Ok(Value::Null),
        Err(_) => Ok(Value::Null),
    }
}
