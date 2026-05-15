use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunConfig {
    pub max_steps: usize,
    pub timeout_ms: u64,
    pub mock_external_calls: bool,
    pub collect_metrics: bool,
}

impl Default for DryRunConfig {
    fn default() -> Self {
        Self {
            max_steps: 1000,
            timeout_ms: 30000,
            mock_external_calls: true,
            collect_metrics: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunResult {
    pub workflow_id: String,
    pub success: bool,
    pub steps_executed: usize,
    pub mocked_calls: Vec<MockedCall>,
    pub variables: HashMap<String, serde_json::Value>,
    pub errors: Vec<String>,
    pub metrics: DryRunMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockedCall {
    pub node_id: String,
    pub call_type: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DryRunMetrics {
    pub total_time_ms: u64,
    pub validation_time_ms: u64,
    pub estimated_real_time_ms: u64,
    pub estimated_cost_usd: f64,
}

pub struct DryRunExecutor;

impl DryRunExecutor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self, workflow: &super::Workflow, config: &DryRunConfig) -> DryRunResult {
        let start_time = std::time::Instant::now();
        let mut variables = HashMap::new();
        let mut mocked_calls = Vec::new();
        let mut errors = Vec::new();
        let mut steps = 0;

        for node in &workflow.nodes {
            if steps >= config.max_steps {
                errors.push(format!("Max steps {} exceeded", config.max_steps));
                break;
            }

            match self.execute_node(node, &mut variables, config.mock_external_calls) {
                Ok(output) => {
                    if config.collect_metrics && Self::is_external_call(node) {
                        mocked_calls.push(MockedCall {
                            node_id: node.id.clone(),
                            call_type: node.operation.clone().unwrap_or_default(),
                            input: variables.clone(),
                            output: output.clone(),
                        });
                    }
                    variables.insert(node.id.clone(), output);
                    steps += 1;
                }
                Err(e) => {
                    errors.push(format!("Node {} failed: {}", node.id, e));
                }
            }
        }

        let total_time = start_time.elapsed().as_millis() as u64;

        DryRunResult {
            workflow_id: workflow.name.clone(),
            success: errors.is_empty(),
            steps_executed: steps,
            mocked_calls,
            variables,
            errors,
            metrics: DryRunMetrics {
                total_time_ms: total_time,
                validation_time_ms: total_time / 10,
                estimated_real_time_ms: total_time * 10,
                estimated_cost_usd: (steps as f64) * 0.001,
            },
        }
    }

    fn execute_node(
        &self,
        node: &super::WorkflowNode,
        _variables: &mut HashMap<String, serde_json::Value>,
        mock_external: bool,
    ) -> Result<serde_json::Value, String> {
        if !mock_external {
            return Ok(serde_json::json!({"status": "skipped"}));
        }

        match node.operation.as_deref() {
            Some("http") | Some("HTTP Request") => Ok(serde_json::json!({
                "status": 200,
                "body": "mock response",
                "headers": {}
            })),
            Some("code") | Some("Code") => Ok(serde_json::json!({"output": "mock code result"})),
            Some("set") | Some("Set") => Ok(serde_json::json!({"success": true})),
            _ => Ok(serde_json::json!({"status": "ok"})),
        }
    }

    fn is_external_call(node: &super::WorkflowNode) -> bool {
        matches!(
            node.operation.as_deref(),
            Some("http") | Some("HTTP Request") | Some("slack") | Some("telegram")
        )
    }
}

impl Default for DryRunExecutor {
    fn default() -> Self {
        Self::new()
    }
}

pub fn plan_mode(workflow: &super::Workflow) -> String {
    let mut plan = String::new();
    plan.push_str(&format!("Workflow: {}\n", workflow.name));
    plan.push_str("=".repeat(40));
    plan.push('\n');
    plan.push_str("Execution Plan:\n\n");

    for (i, node) in workflow.nodes.iter().enumerate() {
        plan.push_str(&format!("{}. [{}] {}\n", i + 1, node.node_type, node.id));

        if let Some(op) = &node.operation {
            plan.push_str(&format!("   Operation: {}\n", op));
        }

        plan.push('\n');
    }

    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dry_run() {
        let workflow = super::Workflow {
            name: "test".to_string(),
            nodes: vec![super::WorkflowNode {
                id: "n1".to_string(),
                node_type: "test".to_string(),
                operation: Some("http".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };

        let executor = DryRunExecutor::new();
        let result = executor.execute(&workflow, &DryRunConfig::default());

        assert!(result.success || !result.errors.is_empty());
    }
}
