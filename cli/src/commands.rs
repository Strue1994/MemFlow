use anyhow::Result;
use reqwest::Client;
use serde_json::Value;

pub struct MemFlowClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl MemFlowClient {
    pub fn new(url: &str, api_key: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            base_url: url.trim_end_matches("/").to_string(),
            api_key: api_key.map(|s| s.to_string()),
        }
    }

    async fn call(&self, method: reqwest::Method, path: &str, body: Option<Value>) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.request(method, &url);
        if let Some(key) = &self.api_key { req = req.header("X-API-Key", key); }
        if let Some(b) = body { req = req.json(&b); }
        let resp = req.send().await?;
        Ok(resp.json().await?)
    }

    pub async fn execute_workflow(&self, id: &str, params: Option<Value>) -> Result<Value> {
        self.call(reqwest::Method::POST, "/execute", Some(serde_json::json!({"workflow_id": id, "params": params}))).await
    }

    pub async fn create_workflow(&self, description: &str) -> Result<Value> {
        self.call(reqwest::Method::POST, "/create_workflow", Some(serde_json::json!({"n8n_json": {"name": description}}))).await
    }

    pub async fn list_workflows(&self) -> Result<Vec<Value>> {
        let val = self.call(reqwest::Method::GET, "/workflows", None).await?;
        Ok(val["workflows"].as_array().cloned().unwrap_or_default())
    }

    pub async fn get_workflow_logs(&self, id: &str, limit: usize) -> Result<Vec<Value>> {
        let val = self.call(reqwest::Method::GET, &format!("/logs/{}?limit={}", id, limit), None).await?;
        Ok(val["items"].as_array().cloned().unwrap_or_default())
    }

    pub async fn get_recent_logs(&self, limit: usize) -> Result<Vec<Value>> {
        let val = self.call(reqwest::Method::GET, &format!("/tasks/history?limit={}", limit), None).await?;
        Ok(val["items"].as_array().cloned().unwrap_or_default())
    }

    pub async fn trigger_learn(&self) -> Result<Value> {
        self.call(reqwest::Method::POST, "/learn", None).await
    }

    pub async fn get_metrics(&self) -> Result<Value> {
        self.call(reqwest::Method::GET, "/metrics", None).await
    }

    pub async fn health_check(&self) -> Result<bool> {
        Ok(self.call(reqwest::Method::GET, "/health", None).await.is_ok())
    }

    pub async fn validate_api_key(&self) -> Result<bool> {
        Ok(self.call(reqwest::Method::GET, "/llm-settings", None).await.is_ok())
    }

    pub fn get_url(&self) -> &str { &self.base_url }
    pub fn get_api_key(&self) -> Option<&str> { self.api_key.as_deref() }
}

// P4.2: Skill CLI commands
pub async fn cmd_skill_create(client: &MemFlowClient, name: &str, desc: &str) -> Result<()> {
    let skill = client.call(reqwest::Method::POST, "/skills", Some(serde_json::json!({
        "name": name, "desc": desc,
    }))).await?;
    println!("Skill created: {}", skill);
    Ok(())
}

pub async fn cmd_skill_list(client: &MemFlowClient) -> Result<()> {
    let skills = client.call(reqwest::Method::GET, "/skills", None).await?;
    println!("{}", serde_json::to_string_pretty(&skills)?);
    Ok(())
}
