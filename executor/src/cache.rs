use crate::error::ExecError;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static MEMCACHE: once_cell::sync::Lazy<Mutex<HashMap<String, (Value, Option<Instant>)>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

pub fn execute_cache_get(key: &str) -> Result<Value, ExecError> {
    let cache = MEMCACHE.lock().unwrap();
    if let Some((val, expires)) = cache.get(key) {
        if let Some(exp) = expires {
            if Instant::now() > *exp { return Ok(Value::Null); }
        }
        return Ok(val.clone());
    }
    Ok(Value::Null)
}

pub fn execute_cache_set(key: &str, value: &Value, ttl: Option<u64>) -> Result<Value, ExecError> {
    let expires = ttl.map(|s| Instant::now() + Duration::from_secs(s));
    MEMCACHE.lock().unwrap().insert(key.to_string(), (value.clone(), expires));
    Ok(serde_json::json!({"status": "cached", "key": key}))
}

pub fn cache_stats() -> usize { MEMCACHE.lock().unwrap().len() }
