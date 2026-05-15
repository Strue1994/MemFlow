use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
}
