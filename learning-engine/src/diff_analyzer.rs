use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffPatch {
    pub op: String,
    pub path: String,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDiff {
    pub id: String,
    pub workflow_id: String,
    pub from_version: u32,
    pub to_version: u32,
    pub diff_patch: Vec<DiffPatch>,
    pub user_id: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationPattern {
    pub pattern_type: String,
    pub description: String,
    pub frequency: u32,
    pub suggested_prompt_modification: String,
}

pub struct DiffAnalyzer {
    patterns: Arc<RwLock<HashMap<String, Vec<ModificationPattern>>>>,
}

impl DiffAnalyzer {
    pub fn new() -> Self {
        Self {
            patterns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn analyze_diffs(&self, diffs: &[WorkflowDiff]) -> HashMap<String, Vec<ModificationPattern>> {
        let mut pattern_counts: HashMap<String, HashMap<String, u32>> = HashMap::new();

        for diff in diffs {
            for patch in &diff.diff_patch {
                let category = self.categorize_patch(patch);
                let workflow_type = self.infer_workflow_type(&diff.workflow_id);
                
                pattern_counts
                    .entry(workflow_type.clone())
                    .or_insert_with(HashMap::new)
                    .entry(category)
                    .and_modify(|c| *c += 1)
                    .or_insert(1);
            }
        }

        let mut results = HashMap::new();
        
        for (workflow_type, patterns) in pattern_counts {
            let mut top_patterns: Vec<ModificationPattern> = patterns
                .into_iter()
                .filter(|(_, count)| *count > 5)
                .map(|(pattern_type, frequency)| {
                    let (description, suggestion) = self.generate_suggestion(&pattern_type, frequency);
                    ModificationPattern {
                        pattern_type: pattern_type.clone(),
                        description,
                        frequency,
                        suggested_prompt_modification: suggestion,
                    }
                })
                .collect();
            
            top_patterns.sort_by(|a, b| b.frequency.cmp(&a.frequency));
            results.insert(workflow_type, top_patterns);
        }

        let mut patterns_guard = self.patterns.write().await;
        *patterns_guard = results.clone();

        results
    }

    fn categorize_patch(&self, patch: &DiffPatch) -> String {
        if patch.path.contains("timeout") {
            "timeout_adjustment".to_string()
        } else if patch.path.contains("url") {
            "url_change".to_string()
        } else if patch.path.contains("headers") {
            "header_modification".to_string()
        } else if patch.path.contains("condition") {
            "condition_change".to_string()
        } else if patch.path.contains("/nodes/") && patch.op == "add" {
            "node_addition".to_string()
        } else if patch.path.contains("/nodes/") && patch.op == "remove" {
            "node_removal".to_string()
        } else if patch.path.contains("/edges/") {
            "connection_change".to_string()
        } else {
            "parameter_tweak".to_string()
        }
    }

    fn infer_workflow_type(&self, workflow_id: &str) -> String {
        if workflow_id.contains("http") || workflow_id.contains("api") {
            "http_workflow".to_string()
        } else if workflow_id.contains("data") || workflow_id.contains("db") {
            "database_workflow".to_string()
        } else if workflow_id.contains("notification") || workflow_id.contains("alert") {
            "notification_workflow".to_string()
        } else {
            "general_workflow".to_string()
        }
    }

    fn generate_suggestion(&self, pattern_type: &str, frequency: u32) -> (String, String) {
        match pattern_type {
            "timeout_adjustment" => (
                format!("Users frequently adjust timeout values ({} times)", frequency),
                "When generating HTTP request workflows, use 30 second timeout unless user explicitly requests fast response (10s)".to_string()
            ),
            "url_change" => (
                format!("Users often modify target URLs ({} times)", frequency),
                "Add a placeholder comment suggesting users verify and update the target URL".to_string()
            ),
            "node_addition" => (
                format!("Users frequently add new nodes ({} times)", frequency),
                "Consider generating with optional error handling nodes that users can remove".to_string()
            ),
            "node_removal" => (
                format!("Users often remove certain nodes ({} times)", frequency),
                "Generate simpler workflows by default, let users add complexity as needed".to_string()
            ),
            "connection_change" => (
                format!("Users modify node connections frequently ({} times)", frequency),
                "Use clear, minimal connections that are easy to understand and modify".to_string()
            ),
            _ => (
                format!("Parameter tweaks ({})", frequency),
                "Provide sensible defaults with clear documentation".to_string()
            )
        }
    }

    pub async fn get_patterns(&self) -> HashMap<String, Vec<ModificationPattern>> {
        self.patterns.read().await.clone()
    }

    pub async fn get_common_patterns(&self, min_frequency: u32) -> Vec<(String, String)> {
        let patterns = self.patterns.read().await;
        let mut common = Vec::new();
        
        for (_workflow_type, workflow_patterns) in patterns.iter() {
            for pattern in workflow_patterns {
                if pattern.frequency >= min_frequency {
                    common.push((pattern.pattern_type.clone(), pattern.suggested_prompt_modification.clone()));
                }
            }
        }
        
        common.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        common
    }
}

impl Default for DiffAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pattern_analysis() {
        let analyzer = DiffAnalyzer::new();
        
        let diffs = vec![
            WorkflowDiff {
                id: "1".to_string(),
                workflow_id: "http_workflow_1".to_string(),
                from_version: 1,
                to_version: 2,
                diff_patch: vec![
                    DiffPatch {
                        op: "replace".to_string(),
                        path: "/nodes/0/params/timeout".to_string(),
                        value: Some(serde_json::json!(30)),
                    }
                ],
                user_id: "user1".to_string(),
                created_at: 1000,
            },
        ];
        
        let results = analyzer.analyze_diffs(&diffs).await;
        assert!(results.contains_key("http_workflow"));
    }
}