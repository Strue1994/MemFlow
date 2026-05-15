use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::diff_analyzer::ModificationPattern;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVersion {
    pub id: i64,
    pub version: String,
    pub content: String,
    pub created_at: i64,
    pub is_active: bool,
    pub ab_test_percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptOptimization {
    pub category: String,
    pub original_prompt: String,
    pub optimized_prompt: String,
    pub expected_improvement: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestResult {
    pub prompt_version_id: i64,
    pub total_generations: u32,
    pub modified_by_user: u32,
    pub modification_rate: f32,
    pub success_rate: f32,
}

pub struct PromptOptimizer {
    current_prompt: Arc<RwLock<String>>,
    versions: Arc<RwLock<Vec<PromptVersion>>>,
    ab_tests: Arc<RwLock<HashMap<i64, ABTestResult>>>,
}

impl PromptOptimizer {
    pub fn new() -> Self {
        let default_prompt = r#"You are a workflow generator for MemFlow, an n8n-compatible execution engine.
Generate JSON workflows that can be parsed by the MemFlow compiler.

## Available Node Types:
- HTTP Request (GET, POST, PUT, DELETE)
- Set Variable
- Code (JavaScript)
- If Condition
- For Loop
- Database Query
- File Operations (Read, Write, Append)
- Send Email (SMTP)
- Slack Message
- Telegram Message
- Google Sheets
- GitHub
- Notion

## Best Practices:
1. Use sensible timeouts (30 seconds for HTTP unless specified)
2. Include error handling where appropriate
3. Keep workflows simple and modular
4. Use meaningful variable names
5. Validate inputs before processing

## Output Format:
Return a valid JSON object with "nodes" and "connections" arrays.
"#;

        Self {
            current_prompt: Arc::new(RwLock::new(default_prompt.to_string())),
            versions: Arc::new(RwLock::new(vec![
                PromptVersion {
                    id: 1,
                    version: "v1.0".to_string(),
                    content: default_prompt.to_string(),
                    created_at: chrono::Utc::now().timestamp(),
                    is_active: true,
                    ab_test_percentage: 100.0,
                }
            ])),
            ab_tests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn optimize_from_patterns(&self, patterns: HashMap<String, Vec<crate::diff_analyzer::ModificationPattern>>) -> Vec<PromptOptimization> {
        let mut optimizations = Vec::new();
        
        let mut prompt_guard = self.current_prompt.write().await;
        let mut current_prompt = prompt_guard.clone();

        for (workflow_type, pattern_list) in patterns {
            if pattern_list.is_empty() {
                continue;
            }

            let optimization = match workflow_type.as_str() {
                "http_workflow" => {
                    let has_timeout = pattern_list.iter().any(|p| p.pattern_type == "timeout_adjustment");
                    if has_timeout {
                        current_prompt = current_prompt.replace(
                            "Use sensible timeouts (30 seconds",
                            "Use 30 second timeouts for HTTP requests by default, unless user explicitly requests fast response (10s)"
                        );
                        Some(PromptOptimization {
                            category: "http_workflow".to_string(),
                            original_prompt: "Use sensible timeouts (30 seconds for HTTP unless specified)".to_string(),
                            optimized_prompt: "Use 30 second timeouts for HTTP requests by default, unless user explicitly requests fast response (10s)".to_string(),
                            expected_improvement: 0.15,
                        })
                    } else {
                        None
                    }
                },
                _ => None,
            };

            if let Some(opt) = optimization {
                optimizations.push(opt);
            }
        }

        *prompt_guard = current_prompt;
        optimizations
    }

    pub async fn create_prompt_version(&self, content: String) -> PromptVersion {
        let mut versions_guard = self.versions.write().await;
        let id = versions_guard.len() as i64 + 1;
        let version = format!("v{}.{}", id, chrono::Utc::now().timestamp());
        
        let new_version = PromptVersion {
            id,
            version: version.clone(),
            content: content.clone(),
            created_at: chrono::Utc::now().timestamp(),
            is_active: false,
            ab_test_percentage: 10.0,
        };
        
        versions_guard.push(new_version.clone());
        
        drop(versions_guard);
        
        self.set_active_prompt(id).await;
        
        new_version
    }

    pub async fn set_active_prompt(&self, version_id: i64) {
        let mut versions_guard = self.versions.write().await;
        for version in versions_guard.iter_mut() {
            version.is_active = version.id == version_id;
            if version.is_active {
                let mut prompt_guard = self.current_prompt.write().await;
                *prompt_guard = version.content.clone();
            }
        }
    }

    pub async fn record_ab_test_result(&self, version_id: i64, result: ABTestResult) {
        let mut tests_guard = self.ab_tests.write().await;
        tests_guard.insert(version_id, result);
    }

    pub async fn should_promote(&self, version_id: i64) -> bool {
        let tests_guard = self.ab_tests.read().await;
        if let Some(result) = tests_guard.get(&version_id) {
            return result.modification_rate < 0.3 && result.success_rate > 0.8;
        }
        false
    }

    pub async fn get_current_prompt(&self) -> String {
        self.current_prompt.read().await.clone()
    }

    pub async fn get_active_version(&self) -> Option<PromptVersion> {
        let versions_guard = self.versions.read().await;
        versions_guard.iter().find(|v| v.is_active).cloned()
    }

    pub async fn list_versions(&self) -> Vec<PromptVersion> {
        self.versions.read().await.clone()
    }

    pub async fn rollback(&self, version_id: i64) -> Result<(), String> {
        let target_content = {
            let versions_guard = self.versions.read().await;
            let target = versions_guard.iter().find(|v| v.id == version_id)
                .ok_or("Version not found")?;
            target.content.clone()
        };
        
        let mut prompt_guard = self.current_prompt.write().await;
        *prompt_guard = target_content;
        
        let mut versions_guard = self.versions.write().await;
        for version in versions_guard.iter_mut() {
            version.is_active = version.id == version_id;
        }
        
        Ok(())
    }

    pub async fn get_ab_test_stats(&self) -> HashMap<i64, ABTestResult> {
        self.ab_tests.read().await.clone()
    }
}

impl Default for PromptOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prompt_creation() {
        let optimizer = PromptOptimizer::new();
        let current = optimizer.get_current_prompt().await;
        assert!(current.contains("MemFlow"));
    }

    #[tokio::test]
    async fn test_version_creation() {
        let optimizer = PromptOptimizer::new();
        let version = optimizer.create_prompt_version("New prompt content".to_string()).await;
        assert!(version.id > 0);
    }
}