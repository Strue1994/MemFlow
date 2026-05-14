use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderKind { OpenAI, Anthropic, Ollama }

#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    async fn complete(&self, messages: &[LLMMessage], tools: &[LLMTool]) -> Result<LLMResponse, LLMError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMessage {
    pub role: String,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMTool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<LLMToolCall>,
    pub tokens_in: u32,
    pub tokens_out: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Rate limited")]
    RateLimited,
    #[error("All providers failed")]
    AllFailed,
}

pub struct OpenAIProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn kind(&self) -> ProviderKind { ProviderKind::OpenAI }

    async fn complete(&self, messages: &[LLMMessage], tools: &[LLMTool]) -> Result<LLMResponse, LLMError> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages.iter().map(|m| {
                let mut msg = serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                });
                if let Some(id) = &m.tool_call_id {
                    msg["tool_call_id"] = serde_json::json!(id);
                }
                msg
            }).collect::<Vec<_>>(),
        });

        let resp = self.client.post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;

        let data: serde_json::Value = resp.json().await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;

        let choice = &data["choices"][0]["message"];
        Ok(LLMResponse {
            content: choice["content"].as_str().map(|s| s.to_string()),
            tool_calls: vec![],
            tokens_in: data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            tokens_out: data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
        })
    }
}

pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self { client: reqwest::Client::new(), api_key: api_key.to_string(), model: model.to_string() }
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn kind(&self) -> ProviderKind { ProviderKind::Anthropic }

    async fn complete(&self, messages: &[LLMMessage], _tools: &[LLMTool]) -> Result<LLMResponse, LLMError> {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 4096,
            "messages": messages.iter().map(|m| serde_json::json!({
                "role": m.role, "content": m.content,
            })).collect::<Vec<_>>(),
        });

        let resp = self.client.post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send().await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;

        let data: serde_json::Value = resp.json().await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;

        Ok(LLMResponse {
            content: data["content"][0]["text"].as_str().map(|s| s.to_string()),
            tool_calls: vec![],
            tokens_in: data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
            tokens_out: data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
        })
    }
}
