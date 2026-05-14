use crate::error::ExecError;
use serde_json::Value;

pub fn execute_notion_create_page(db: &str, properties: &Value, token: &str) -> Result<Value, ExecError> {
    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({"parent": {"type": "database_id", "database_id": db}, "properties": properties});
    let resp = client.post("https://api.notion.com/v1/pages")
        .header("Authorization", format!("Bearer {}", token))
        .header("Notion-Version", "2022-06-28")
        .json(&body).send()
        .map_err(|e| ExecError::HttpError(format!("Notion: {}", e)))?;
    if !resp.status().is_success() { return Err(ExecError::HttpError(format!("Notion HTTP {}", resp.status()))); }
    let r: Value = resp.json().map_err(|e| ExecError::HttpError(format!("JSON: {}", e)))?;
    Ok(serde_json::json!({"status": "created", "page_id": r["id"]}))
}

pub fn execute_notion_query(db: &str, filter: &Option<Value>, token: &str) -> Result<Value, ExecError> {
    let client = reqwest::blocking::Client::new();
    let mut body = serde_json::json!({});
    if let Some(f) = filter { body["filter"] = f.clone(); }
    let resp = client.post(&format!("https://api.notion.com/v1/databases/{}/query", db))
        .header("Authorization", format!("Bearer {}", token))
        .header("Notion-Version", "2022-06-28")
        .json(&body).send()
        .map_err(|e| ExecError::HttpError(format!("Notion: {}", e)))?;
    if !resp.status().is_success() { return Err(ExecError::HttpError(format!("Notion HTTP {}", resp.status()))); }
    resp.json().map_err(|e| ExecError::HttpError(format!("JSON: {}", e)))
}
