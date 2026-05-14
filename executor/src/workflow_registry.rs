use crate::db::WorkflowDb;
use crate::Workflow;
use once_cell::sync::OnceCell;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::RwLock;

static DB: OnceCell<WorkflowDb> = OnceCell::new();
static CACHE: OnceCell<RwLock<HashMap<(String, u32), Workflow>>> = OnceCell::new();
static N8N_JSON_CACHE: OnceCell<RwLock<HashMap<(String, u32), String>>> = OnceCell::new();

pub fn init(db_path: &std::path::Path) -> Result<(), anyhow::Error> {
    let db = WorkflowDb::open(db_path)?;
    DB.set(db)
        .map_err(|_| anyhow::anyhow!("DB already initialized"))?;
    CACHE
        .set(RwLock::new(HashMap::new()))
        .map_err(|_| anyhow::anyhow!("Cache already initialized"))?;
    N8N_JSON_CACHE
        .set(RwLock::new(HashMap::new()))
        .map_err(|_| anyhow::anyhow!("JSON cache already initialized"))?;
    Ok(())
}

pub fn is_initialized() -> bool {
    DB.get().is_some()
}

pub fn register_workflow(
    id: &str,
    name: &str,
    n8n_json: &JsonValue,
    workflow: Workflow,
) -> Result<u32, anyhow::Error> {
    let db = DB.get().ok_or(anyhow::anyhow!("DB not initialized"))?;
    let version = db.save_workflow(id, name, n8n_json, &workflow)?;

    let cache = CACHE
        .get()
        .ok_or(anyhow::anyhow!("Cache not initialized"))?;
    let mut cache_map = cache
        .write()
        .map_err(|_| anyhow::anyhow!("Cache write lock failed"))?;
    cache_map.insert((id.to_string(), version), workflow);

    let json_cache = N8N_JSON_CACHE
        .get()
        .ok_or(anyhow::anyhow!("JSON cache not initialized"))?;
    let mut json_map = json_cache
        .write()
        .map_err(|_| anyhow::anyhow!("JSON cache write lock failed"))?;
    json_map.insert((id.to_string(), version), n8n_json.to_string());

    Ok(version)
}

pub fn get_workflow(id: &str, version: Option<u32>) -> Option<Workflow> {
    let cache = CACHE.get()?;
    let latest_version = version.or_else(|| {
        let db = DB.get()?;
        db.list_versions(id).ok()?.into_iter().next()
    })?;

    {
        let cache_map = cache.read().ok()?;
        if let Some(wf) = cache_map.get(&(id.to_string(), latest_version)) {
            return Some(wf.clone());
        }
    }
    let db = DB.get()?;
    let wf = db
        .load_workflow(id, version.or(Some(latest_version)))
        .ok()??;
    let mut cache_map = cache.write().ok()?;
    cache_map.insert((id.to_string(), latest_version), wf.clone());
    Some(wf)
}

pub fn get_n8n_json(id: &str, version: Option<u32>) -> Option<String> {
    let json_cache = N8N_JSON_CACHE.get()?;
    let latest_version = version.or_else(|| {
        let db = DB.get()?;
        db.list_versions(id).ok()?.into_iter().next()
    })?;

    {
        let cache_map = json_cache.read().ok()?;
        if let Some(json) = cache_map.get(&(id.to_string(), latest_version)) {
            return Some(json.clone());
        }
    }
    let db = DB.get()?;
    let json = db
        .load_n8n_json(id, version.or(Some(latest_version)))
        .ok()??;
    let mut json_map = json_cache.write().ok()?;
    json_map.insert((id.to_string(), latest_version), json.clone());
    Some(json)
}

pub fn get_workflow_metadata(id: &str, version: Option<u32>) -> Option<(String, u32)> {
    let db = DB.get()?;
    db.load_workflow_metadata(id, version).ok()?
}

pub fn list_workflows() -> Vec<(String, String, u32)> {
    let db = match DB.get() {
        Some(d) => d,
        None => return Vec::new(),
    };
    db.list_workflows().ok().unwrap_or_default()
}

pub fn list_versions(id: &str) -> Vec<u32> {
    let db = match DB.get() {
        Some(d) => d,
        None => return Vec::new(),
    };
    db.list_versions(id).ok().unwrap_or_default()
}

pub fn rollback(id: &str) -> Result<Option<u32>, anyhow::Error> {
    let db = DB.get().ok_or(anyhow::anyhow!("DB not initialized"))?;
    let new_version = db.rollback(id)?;

    if let Some(ver) = new_version {
        if let Some(cache) = CACHE.get() {
            if let Ok(mut write) = cache.write() {
                write.remove(&(id.to_string(), ver + 1));
            }
        }
        if let Some(json_cache) = N8N_JSON_CACHE.get() {
            if let Ok(mut write) = json_cache.write() {
                write.remove(&(id.to_string(), ver + 1));
            }
        }
    }

    Ok(new_version)
}

pub fn get_db() -> Option<&'static WorkflowDb> {
    DB.get()
}
