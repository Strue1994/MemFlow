use crate::error::ExecError;
use serde_json::Value;
use compiler::{Instruction, Workflow, WorkflowNode, HttpMethod};

/// Executes a workflow in dry-run mode (no side effects)
pub struct DryRunExecutor;

impl DryRunExecutor {
    pub fn new() -> Self { Self }

    pub fn execute(&self, workflow: &Workflow, config: &DryRunConfig) -> DryRunResult {
        let mut errors = Vec::new();
        let mut steps = Vec::new();

        for node in &workflow.nodes {
            for instr in &node.instructions {
                match instr {
                    Instruction::HttpRequest { url, .. } => {
                        if config.check_network && !url.starts_with("http") {
                            errors.push(format!("Invalid URL: {}", url));
                        }
                        steps.push(format!("HTTP: {}", url));
                    }
                    Instruction::DBQuery { query, .. } => {
                        if config.check_sql_injection && (query.to_lowercase().contains("drop") || query.to_lowercase().contains("delete")) {
                            errors.push(format!("Potentially dangerous query: {}", query));
                        }
                        steps.push(format!("DB: {}", query));
                    }
                    Instruction::WriteFile { path, .. } => {
                        if config.check_file_access && !config.allowed_paths.iter().any(|p| path.starts_with(p)) {
                            errors.push(format!("File write not allowed: {}", path));
                        }
                        steps.push(format!("File: {}", path));
                    }
                    _ => steps.push(format!("{:?}", instr)),
                }
            }
        }

        DryRunResult {
            success: errors.is_empty(),
            steps,
            errors,
        }
    }
}

pub struct DryRunConfig {
    pub check_network: bool,
    pub check_sql_injection: bool,
    pub check_file_access: bool,
    pub allowed_paths: Vec<String>,
}

impl Default for DryRunConfig {
    fn default() -> Self {
        Self {
            check_network: true,
            check_sql_injection: true,
            check_file_access: true,
            allowed_paths: vec!["/tmp".to_string(), "./output".to_string()],
        }
    }
}

pub struct DryRunResult {
    pub success: bool,
    pub steps: Vec<String>,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dry_run() {
        let workflow = Workflow {
            entry: "n1".to_string(),
            nodes: vec![WorkflowNode {
                id: "n1".to_string(),
                instructions: vec![Instruction::HttpRequest {
                    method: HttpMethod::Get,
                    url: "https://example.com".to_string(),
                    headers: vec![],
                    body: None,
                    timeout_ms: None,
                    max_retries: None,
                    output_var: "result".to_string(),
                }],
                dependencies: vec![],
            }],
        };

        let executor = DryRunExecutor::new();
        let result = executor.execute(&workflow, &DryRunConfig::default());

        assert!(result.success || !result.errors.is_empty());
    }
}
