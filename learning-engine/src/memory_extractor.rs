use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExtract {
    pub id: String,
    pub workflow_id: String,
    pub extract_type: ExtractType,
    pub content: String,
    pub importance: f32,
    pub metadata: HashMap<String, String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractType {
    Pattern,
    Error,
    Success,
    Parameter,
    Config,
}

pub struct MemoryExtractor;

impl MemoryExtractor {
    pub fn extract_from_execution(
        workflow_id: &str,
        execution_result: &ExecutionResult,
    ) -> Vec<MemoryExtract> {
        let mut extracts = Vec::new();

        if let Some(errors) = &execution_result.errors {
            for error in errors {
                extracts.push(MemoryExtract {
                    id: format!("extract_{}", uuid::Uuid::new_v4()),
                    workflow_id: workflow_id.to_string(),
                    extract_type: ExtractType::Error,
                    content: error.clone(),
                    importance: 0.9,
                    metadata: HashMap::from([("source".to_string(), "execution".to_string())]),
                    created_at: chrono::Utc::now().timestamp(),
                });
            }
        }

        if execution_result.success {
            extracts.push(MemoryExtract {
                id: format!("extract_{}", uuid::Uuid::new_v4()),
                workflow_id: workflow_id.to_string(),
                extract_type: ExtractType::Success,
                content: format!("Workflow {} completed successfully", workflow_id),
                importance: 0.7,
                metadata: HashMap::new(),
                created_at: chrono::Utc::now().timestamp(),
            });
        }

        extracts
    }

    pub fn extract_patterns(workflow_json: &serde_json::Value) -> Vec<MemoryExtract> {
        let mut extracts = Vec::new();

        if let Some(nodes) = workflow_json["nodes"].as_array() {
            let node_types: Vec<String> = nodes
                .iter()
                .filter_map(|n| n["type"].as_str().map(|s| s.to_string()))
                .collect();

            if !node_types.is_empty() {
                extracts.push(MemoryExtract {
                    id: format!("pattern_{}", uuid::Uuid::new_v4()),
                    workflow_id: "template".to_string(),
                    extract_type: ExtractType::Pattern,
                    content: format!("Used nodes: {:?}", node_types),
                    importance: 0.5,
                    metadata: HashMap::from([("type".to_string(), "node_sequence".to_string())]),
                    created_at: chrono::Utc::now().timestamp(),
                });
            }
        }

        extracts
    }

    pub fn extract_parameters(params: &HashMap<String, serde_json::Value>) -> Vec<MemoryExtract> {
        let mut extracts = Vec::new();

        for (key, value) in params {
            extracts.push(MemoryExtract {
                id: format!("param_{}", uuid::Uuid::new_v4()),
                workflow_id: "template".to_string(),
                extract_type: ExtractType::Parameter,
                content: format!("{}: {}", key, value),
                importance: 0.4,
                metadata: HashMap::from([("key".to_string(), key.clone())]),
                created_at: chrono::Utc::now().timestamp(),
            });
        }

        extracts
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: Option<String>,
    pub errors: Option<Vec<String>>,
    pub duration_ms: i64,
}

pub struct AutoMemoryExtractor {
    history: Arc<RwLock<Vec<ConversationEntry>>>,
    llm_endpoint: String,
    api_key: Option<String>,
}

impl AutoMemoryExtractor {
    pub fn new(llm_endpoint: String, api_key: Option<String>) -> Self {
        Self {
            history: Arc::new(RwLock::new(Vec::new())),
            llm_endpoint,
            api_key,
        }
    }

    pub async fn add_conversation(&self, user_input: &str, agent_response: &str) {
        let entry = ConversationEntry {
            user_input: user_input.to_string(),
            agent_response: agent_response.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            processed: false,
        };
        self.history.write().await.push(entry);
    }

    pub async fn extract_from_history(&self) -> Vec<ExtractedMemory> {
        let (timestamps, prompt) = {
            let history = self.history.read().await;
            let unprocessed: Vec<_> = history.iter().filter(|e| !e.processed).collect();

            if unprocessed.is_empty() {
                return Vec::new();
            }

            let prompt = self.build_extraction_prompt(&unprocessed);
            let timestamps: Vec<i64> = unprocessed.iter().map(|e| e.timestamp).collect();
            (timestamps, prompt)
        };

        let extracts = self.call_llm(&prompt).await;

        let mut history = self.history.write().await;
        for ts in &timestamps {
            if let Some(e) = history.iter_mut().find(|h| h.timestamp == *ts) {
                e.processed = true;
            }
        }

        extracts
    }

    fn build_extraction_prompt(&self, entries: &[&ConversationEntry]) -> String {
        let mut prompt = "从以下对话中提取用户偏好、重复需求、纠正过的错误。\n".to_string();
        prompt.push_str("输出JSON数组，每个元素包含 type(preference|error|correction), content, importance(0-1):\n\n");

        for entry in entries {
            prompt.push_str(&format!("用户: {}\n助手: {}\n\n", entry.user_input, entry.agent_response));
        }

        prompt
    }

    async fn call_llm(&self, prompt: &str) -> Vec<ExtractedMemory> {
        let client = reqwest::Client::new();
        
        let body = serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.3,
            "max_tokens": 1000
        });

        let mut request = client.post(&self.llm_endpoint)
            .header("Content-Type", "application/json");
        
        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        match request.json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if let Some(content) = data.get("choices").and_then(|c| c.as_array())
                        .and_then(|c| c.first())
                        .and_then(|c| c.get("message"))
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_str())
                    {
                        return self.parse_llm_response(content);
                    }
                }
            }
            _ => {}
        }

        Vec::new()
    }

    fn parse_llm_response(&self, content: &str) -> Vec<ExtractedMemory> {
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(content) {
            arr.into_iter().filter_map(|v| {
                Some(ExtractedMemory {
                    memory_type: v.get("type")?.as_str()?.to_string(),
                    content: v.get("content")?.as_str()?.to_string(),
                    importance: v.get("importance")
                        .and_then(|i| i.as_f64())
                        .unwrap_or(0.5) as f32,
                    source: "llm_extraction".to_string(),
                    confirmed: false,
                })
            }).collect()
        } else {
            Vec::new()
        }
    }

    pub async fn schedule_periodic_extraction(&self, interval_hours: u32) {
        use tokio::time::{interval, Duration};
        
        let mut ticker = interval(Duration::from_secs(interval_hours as u64 * 3600));
        
        loop {
            ticker.tick().await;
            self.extract_from_history().await;
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConversationEntry {
    pub user_input: String,
    pub agent_response: String,
    pub timestamp: i64,
    pub processed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMemory {
    pub memory_type: String,
    pub content: String,
    pub importance: f32,
    pub source: String,
    pub confirmed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_from_result() {
        let result = ExecutionResult {
            success: true,
            output: Some("done".to_string()),
            errors: None,
            duration_ms: 100,
        };

        let extracts = MemoryExtractor::extract_from_execution("wf1", &result);
        assert!(!extracts.is_empty());
    }

    #[test]
    fn test_parse_llm_response() {
        let extractor = AutoMemoryExtractor::new(
            "http://localhost:3000/v1/chat/completions".to_string(),
            None
        );
        
        let response = r#"[{"type":"preference","content":"用户喜欢JSON格式","importance":0.8}]"#;
        let memory = extractor.parse_llm_response(response);
        
        assert_eq!(memory.len(), 1);
        assert_eq!(memory[0].memory_type, "preference");
    }
}
