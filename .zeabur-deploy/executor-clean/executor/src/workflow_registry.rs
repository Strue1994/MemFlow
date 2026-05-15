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
    CACHE.set(RwLock::new(HashMap::new())).unwrap();
    N8N_JSON_CACHE.set(RwLock::new(HashMap::new())).unwrap();
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

    let cache = CACHE.get().unwrap();
    let mut cache_map = cache.write().unwrap();
    cache_map.insert((id.to_string(), version), workflow);

    let json_cache = N8N_JSON_CACHE.get().unwrap();
    let mut json_map = json_cache.write().unwrap();
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
        let cache_map = cache.read().unwrap();
        if let Some(wf) = cache_map.get(&(id.to_string(), latest_version)) {
            return Some(wf.clone());
        }
    }
    let db = DB.get()?;
    let wf = db
        .load_workflow(id, version.or(Some(latest_version)))
        .ok()??;
    let mut cache_map = cache.write().unwrap();
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
        let cache_map = json_cache.read().unwrap();
        if let Some(json) = cache_map.get(&(id.to_string(), latest_version)) {
            return Some(json.clone());
        }
    }
    let db = DB.get()?;
    let json = db
        .load_n8n_json(id, version.or(Some(latest_version)))
        .ok()??;
    let mut json_map = json_cache.write().unwrap();
    json_map.insert((id.to_string(), latest_version), json.clone());
    Some(json)
}

pub fn get_workflow_metadata(id: &str, version: Option<u32>) -> Option<(String, u32)> {
    let db = DB.get()?;
    db.load_workflow_metadata(id, version).ok()?
}

pub fn list_workflows() -> Vec<(String, String, u32)> {
    let db = DB.get().expect("DB not initialized");
    db.list_workflows().unwrap_or_default()
}

pub fn list_versions(id: &str) -> Vec<u32> {
    let db = DB.get().expect("DB not initialized");
    db.list_versions(id).unwrap_or_default()
}

pub fn rollback(id: &str) -> Result<Option<u32>, anyhow::Error> {
    let db = DB.get().ok_or(anyhow::anyhow!("DB not initialized"))?;
    let new_version = db.rollback(id)?;

    if let Some(ver) = new_version {
        CACHE
            .get()
            .unwrap()
            .write()
            .unwrap()
            .remove(&(id.to_string(), ver + 1));
        N8N_JSON_CACHE
            .get()
            .unwrap()
            .write()
            .unwrap()
            .remove(&(id.to_string(), ver + 1));
    }

    Ok(new_version)
}

pub fn get_db() -> Option<&'static WorkflowDb> {
    DB.get()
}
