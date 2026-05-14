use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub rule_id: String,
    pub severity: String,
    pub message: String,
    pub suggestion: Option<String>,
    pub node_id: Option<String>,
}

pub struct WorkflowValidator {
    rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
struct Rule {
    id: String,
    category: String,
    severity: String,
    description: String,
    check_fn: fn(&Value) -> Option<String>,
    fix: Option<String>,
}

impl WorkflowValidator {
    pub fn new() -> Self {
        let mut validator = Self { rules: vec![] };
        validator.load_builtin_rules();
        validator
    }

    fn load_builtin_rules(&mut self) {
        self.rules.push(Rule {
            id: "R001".to_string(),
            category: "structure".to_string(),
            severity: "error".to_string(),
            description: "工作流必须至少有一个触发器节点".to_string(),
            check_fn: |wf| {
                let nodes = wf.get("nodes").and_then(|n| n.as_array());
                let has_trigger = nodes
                    .map(|arr| {
                        arr.iter().any(|n| {
                            n.get("type")
                                .and_then(|t| t.as_str())
                                .map(|t| t.contains("Trigger"))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false);

                if !has_trigger {
                    Some("缺少触发器节点，请添加 Schedule Trigger 或 Webhook Trigger".to_string())
                } else {
                    None
                }
            },
            fix: Some("添加 Schedule Trigger 或 Webhook Trigger 节点".to_string()),
        });

        self.rules.push(Rule {
            id: "R002".to_string(),
            category: "http".to_string(),
            severity: "warning".to_string(),
            description: "HTTP Request 节点未设置超时时间".to_string(),
            check_fn: |wf| {
                let nodes = wf.get("nodes").and_then(|n| n.as_array());
                if let Some(nodes) = nodes {
                    for node in nodes {
                        if let Some(node_type) = node.get("type").and_then(|t| t.as_str()) {
                            if node_type.contains("httpRequest") {
                                let has_timeout = node
                                    .get("parameters")
                                    .and_then(|p| p.get("timeout"))
                                    .is_some();
                                if !has_timeout {
                                    let node_name = node
                                        .get("name")
                                        .and_then(|n| n.as_str())
                                        .unwrap_or("unknown");
                                    return Some(format!("节点 '{}' 未设置超时时间", node_name));
                                }
                            }
                        }
                    }
                }
                None
            },
            fix: Some("设置 timeout 为 30000 (30秒)".to_string()),
        });

        self.rules.push(Rule {
            id: "R004".to_string(),
            category: "connection".to_string(),
            severity: "error".to_string(),
            description: "每个 Webhook 节点必须有对应的 Respond to Webhook 节点".to_string(),
            check_fn: |wf| {
                let nodes = wf.get("nodes").and_then(|n| n.as_array());
                if let Some(nodes) = nodes {
                    let has_webhook = nodes.iter().any(|n| {
                        n.get("type")
                            .and_then(|t| t.as_str())
                            .map(|t| t.contains("webhook") && !t.contains("Respond"))
                            .unwrap_or(false)
                    });

                    let has_respond = nodes.iter().any(|n| {
                        n.get("type")
                            .and_then(|t| t.as_str())
                            .map(|t| t.contains("RespondToWebhook"))
                            .unwrap_or(false)
                    });

                    if has_webhook && !has_respond {
                        return Some("Webhook 节点缺少 Respond to Webhook 节点".to_string());
                    }
                }
                None
            },
            fix: Some("在 Webhook 后添加 Respond to Webhook 节点".to_string()),
        });

        self.rules.push(Rule {
            id: "R005".to_string(),
            category: "http".to_string(),
            severity: "error".to_string(),
            description: "HTTP Request 节点不能同时设置 body 和 form-data".to_string(),
            check_fn: |wf| {
                let nodes = wf.get("nodes").and_then(|n| n.as_array());
                if let Some(nodes) = nodes {
                    for node in nodes {
                        if let Some(node_type) = node.get("type").and_then(|t| t.as_str()) {
                            if node_type.contains("httpRequest") {
                                let params = node.get("parameters");
                                let has_body = params.and_then(|p| p.get("body")).is_some();
                                let has_form = params
                                    .and_then(|p| p.get("bodyContentType"))
                                    .and_then(|b| b.as_str())
                                    .map(|b| b.contains("form-data"))
                                    .unwrap_or(false);

                                if has_body && has_form {
                                    let node_name = node
                                        .get("name")
                                        .and_then(|n| n.as_str())
                                        .unwrap_or("unknown");
                                    return Some(format!(
                                        "节点 '{}' 同时设置了 body 和 form-data",
                                        node_name
                                    ));
                                }
                            }
                        }
                    }
                }
                None
            },
            fix: Some("移除 body 或 form-data 中的一个".to_string()),
        });

        self.rules.push(Rule {
            id: "R007".to_string(),
            category: "slack".to_string(),
            severity: "error".to_string(),
            description: "Slack 节点缺少必需的 channel 参数".to_string(),
            check_fn: |wf| {
                let nodes = wf.get("nodes").and_then(|n| n.as_array());
                if let Some(nodes) = nodes {
                    for node in nodes {
                        if let Some(node_type) = node.get("type").and_then(|t| t.as_str()) {
                            if node_type.contains("slack") {
                                let has_channel = node
                                    .get("parameters")
                                    .and_then(|p| p.get("channel"))
                                    .is_some();
                                if !has_channel {
                                    let node_name = node
                                        .get("name")
                                        .and_then(|n| n.as_str())
                                        .unwrap_or("unknown");
                                    return Some(format!(
                                        "Slack 节点 '{}' 缺少 channel 参数",
                                        node_name
                                    ));
                                }
                            }
                        }
                    }
                }
                None
            },
            fix: Some("添加 channel 参数 (如 #general)".to_string()),
        });

        self.rules.push(Rule {
            id: "R008".to_string(),
            category: "http".to_string(),
            severity: "error".to_string(),
            description: "HTTP Request 节点缺少必需的 url 参数".to_string(),
            check_fn: |wf| {
                let nodes = wf.get("nodes").and_then(|n| n.as_array());
                if let Some(nodes) = nodes {
                    for node in nodes {
                        if let Some(node_type) = node.get("type").and_then(|t| t.as_str()) {
                            if node_type.contains("httpRequest") {
                                let has_url =
                                    node.get("parameters").and_then(|p| p.get("url")).is_some();
                                if !has_url {
                                    let node_name = node
                                        .get("name")
                                        .and_then(|n| n.as_str())
                                        .unwrap_or("unknown");
                                    return Some(format!(
                                        "HTTP Request 节点 '{}' 缺少 url 参数",
                                        node_name
                                    ));
                                }
                            }
                        }
                    }
                }
                None
            },
            fix: Some("添加 url 参数".to_string()),
        });

        self.rules.push(Rule {
            id: "R010".to_string(),
            category: "structure".to_string(),
            severity: "error".to_string(),
            description: "工作流节点数量为 0".to_string(),
            check_fn: |wf| {
                let nodes = wf.get("nodes").and_then(|n| n.as_array());
                if let Some(nodes) = nodes {
                    if nodes.is_empty() {
                        return Some("工作流没有节点".to_string());
                    }
                } else {
                    return Some("工作流缺少 nodes 字段".to_string());
                }
                None
            },
            fix: Some("添加至少一个节点".to_string()),
        });

        self.rules.push(Rule {
            id: "R015".to_string(),
            category: "error_handling".to_string(),
            severity: "warning".to_string(),
            description: "工作流缺少错误处理节点".to_string(),
            check_fn: |wf| {
                let nodes = wf.get("nodes").and_then(|n| n.as_array());
                if let Some(nodes) = nodes {
                    let has_error_handling = nodes.iter().any(|n| {
                        n.get("type")
                            .and_then(|t| t.as_str())
                            .map(|t| t.contains("Error") || t.contains("Catch"))
                            .unwrap_or(false)
                    });
                    if !has_error_handling {
                        return Some("工作流缺少错误处理节点".to_string());
                    }
                }
                None
            },
            fix: Some("添加 Error Workflow 或 Try/Catch 节点".to_string()),
        });
    }

    pub fn validate(&self, workflow_json: &Value) -> Vec<ValidationIssue> {
        let mut issues = vec![];

        for rule in &self.rules {
            if let Some(error_msg) = (rule.check_fn)(workflow_json) {
                issues.push(ValidationIssue {
                    rule_id: rule.id.clone(),
                    severity: rule.severity.clone(),
                    message: error_msg,
                    suggestion: rule.fix.clone(),
                    node_id: None,
                });
            }
        }

        issues
    }

    pub fn validate_with_fix(
        &self,
        workflow_json: &Value,
        max_iterations: usize,
    ) -> (Vec<ValidationIssue>, Value) {
        let current = workflow_json.clone();
        let mut all_issues = vec![];
        let mut iterations = 0;

        loop {
            let issues = self.validate(&current);
            if issues.is_empty() || iterations >= max_iterations {
                break;
            }

            all_issues.extend(issues);
            iterations += 1;
        }

        (all_issues, current)
    }
}

impl Default for WorkflowValidator {
    fn default() -> Self {
        Self::new()
    }
}

pub fn validate_workflow_json(workflow_json: &Value) -> Vec<ValidationIssue> {
    let validator = WorkflowValidator::new();
    validator.validate(workflow_json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_missing_trigger() {
        let validator = WorkflowValidator::new();
        let workflow = json!({
            "nodes": [
                { "id": "1", "name": "HTTP Request", "type": "n8n-nodes-base.httpRequest", "parameters": {} }
            ]
        });

        let issues = validator.validate(&workflow);
        assert!(issues.iter().any(|i| i.rule_id == "R001"));
    }

    #[test]
    fn test_validate_with_trigger() {
        let validator = WorkflowValidator::new();
        let workflow = json!({
            "nodes": [
                { "id": "1", "name": "Schedule", "type": "n8n-nodes-base.scheduleTrigger", "parameters": {} }
            ]
        });

        let issues = validator.validate(&workflow);
        assert!(!issues.iter().any(|i| i.rule_id == "R001"));
    }

    #[test]
    fn test_validate_missing_url() {
        let validator = WorkflowValidator::new();
        let workflow = json!({
            "nodes": [
                { "id": "1", "name": "HTTP", "type": "n8n-nodes-base.httpRequest", "parameters": {} }
            ]
        });

        let issues = validator.validate(&workflow);
        assert!(issues.iter().any(|i| i.rule_id == "R008"));
    }
}
