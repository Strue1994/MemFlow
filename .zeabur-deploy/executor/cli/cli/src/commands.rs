use reqwest::Client;
use reqwest::StatusCode;
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),
}

pub struct MemFlowClient {
    client: Client,
    url: String,
    api_key: Option<String>,
}

impl MemFlowClient {
    pub fn new(url: &str, api_key: Option<&str>) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(key) = api_key {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", key).parse().unwrap(),
            );
        }

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            url: url.to_string(),
            api_key: api_key.map(|s| s.to_string()),
        }
    }

    pub async fn execute_workflow(
        &self,
        workflow_id: &str,
        params: Option<Value>,
    ) -> Result<Value, CliError> {
        let response = self
            .client
            .post(format!("{}/execute", self.url))
            .json(&serde_json::json!({
                "workflow_id": workflow_id,
                "params": params.unwrap_or(Value::Null)
            }))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error: Value = response.json().await.unwrap_or_default();
            Err(CliError::Api(
                error.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error").to_string()
            ))
        }
    }

    pub async fn create_workflow(&self, description: &str) -> Result<Value, CliError> {
        let response = self
            .client
            .post(format!("{}/generate", self.url))
            .json(&serde_json::json!({
                "prompt": description
            }))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error: Value = response.json().await.unwrap_or_default();
            Err(CliError::Api(
                error.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error").to_string()
            ))
        }
    }

    pub async fn list_workflows(&self) -> Result<Vec<Value>, CliError> {
        let response = self
            .client
            .get(format!("{}/workflows", self.url))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error: Value = response.json().await.unwrap_or_default();
            Err(CliError::Api(
                error.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error").to_string()
            ))
        }
    }

    pub async fn get_workflow_logs(&self, workflow_id: &str, limit: usize) -> Result<Vec<Value>, CliError> {
        let response = self
            .client
            .get(format!("{}/workflow/{}/logs", self.url, workflow_id))
            .query(&[("limit", limit.to_string())])
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error: Value = response.json().await.unwrap_or_default();
            Err(CliError::Api(
                error.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error").to_string()
            ))
        }
    }

    pub async fn get_recent_logs(&self, limit: usize) -> Result<Vec<Value>, CliError> {
        let response = self
            .client
            .get(format!("{}/logs", self.url))
            .query(&[("limit", limit.to_string())])
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error: Value = response.json().await.unwrap_or_default();
            Err(CliError::Api(
                error.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error").to_string()
            ))
        }
    }

    pub async fn trigger_learn(&self) -> Result<Value, CliError> {
        let response = self
            .client
            .post(format!("{}/learn", self.url))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error: Value = response.json().await.unwrap_or_default();
            Err(CliError::Api(
                error.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error").to_string()
            ))
        }
    }

    pub async fn get_metrics(&self) -> Result<Value, CliError> {
        let response = self
            .client
            .get(format!("{}/metrics", self.url))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.text().await?.parse().unwrap_or(Value::Object(serde_json::Map::new())))
        } else {
            let error: Value = response.json().await.unwrap_or_default();
            Err(CliError::Api(
                error.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error").to_string()
            ))
        }
    }

    pub async fn health_check(&self) -> Result<bool, CliError> {
        let response = self
            .client
            .get(&self.url)
            .send()
            .await?;
        Ok(response.status().is_success())
    }

    pub async fn validate_api_key(&self) -> Result<bool, CliError> {
        let response = self
            .client
            .get(format!("{}/workflows", self.url))
            .send()
            .await?;
        Ok(response.status() != StatusCode::UNAUTHORIZED)
    }

    pub fn get_url(&self) -> &str {
        &self.url
    }

    pub fn get_api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }
}