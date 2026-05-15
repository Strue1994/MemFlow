use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nValidationError {
    pub error_type: String,
    pub node: Option<String>,
    pub property: Option<String>,
    pub message: String,
    pub severity: String,
    pub auto_fix: bool,
    pub fix_suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nWorkflowValidation {
    pub workflow_id: String,
    pub valid: bool,
    pub errors: Vec<N8nValidationError>,
    pub warnings: Vec<String>,
}

pub struct N8nValidator {
    rules: Vec<ValidationRule>,
}

#[derive(Debug, Clone)]
struct ValidationRule {
    id: String,
    error_type: String,
    severity: String,
    check: fn(&N8nWorkflow) -> Option<N8nValidationError>,
}

impl N8nValidator {
    pub fn new() -> Self {
        let rules = vec![
            ValidationRule {
                id: "webhook_response".to_string(),
                error_type: "missing_response".to_string(),
                severity: "error".to_string(),
                check: |wf| Self::check_webhook_has_response(wf),
            },
            ValidationRule {
                id: "http_body_form".to_string(),
                error_type: "invalid_value".to_string(),
                severity: "error".to_string(),
                check: |wf| Self::check_http_no_body_and_form(wf),
            },
            ValidationRule {
                id: "if_branches".to_string(),
                error_type: "incomplete_branch".to_string(),
                severity: "warning".to_string(),
                check: |wf| Self::check_if_branches_connected(wf),
            },
            ValidationRule {
                id: "required_fields".to_string(),
                error_type: "missing_required".to_string(),
                severity: "error".to_string(),
                check: |wf| Self::check_required_fields(wf),
            },
            ValidationRule {
                id: "expression_syntax".to_string(),
                error_type: "invalid_expression".to_string(),
                severity: "error".to_string(),
                check: |wf| Self::check_expression_syntax(wf),
            },
        ];
        Self { rules }
    }

    pub fn validate(&self, workflow: &N8nWorkflow) -> N8nWorkflowValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for rule in &self.rules {
            if let Some(error) = (rule.check)(workflow) {
                if error.severity == "error" {
                    errors.push(error);
                } else {
                    warnings.push(error.message);
                }
            }
        }

        N8nWorkflowValidation {
            workflow_id: workflow.name.clone(),
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    fn check_webhook_has_response(wf: &N8nWorkflow) -> Option<N8nValidationError> {
        let has_webhook = wf.nodes.iter().any(|n| n.type_.contains("webhook"));
        let has_response = wf
            .nodes
            .iter()
            .any(|n| n.type_.contains("RespondToWebhook"));

        if has_webhook && !has_response {
            return Some(N8nValidationError {
                error_type: "missing_response".to_string(),
                node: None,
                property: None,
                message: "Webhook node exists but no Respond to Webhook node found".to_string(),
                severity: "error".to_string(),
                auto_fix: false,
                fix_suggestion: Some(
                    "Add a Respond to Webhook node after the webhook trigger".to_string(),
                ),
            });
        }
        None
    }

    fn check_http_no_body_and_form(wf: &N8nWorkflow) -> Option<N8nValidationError> {
        for node in &wf.nodes {
            if node.type_.contains("HttpRequest") || node.type_.contains("HTTP Request") {
                let parameters = &node.parameters;
                let has_body = parameters.contains("\"body\"") || parameters.contains("'body'");
                let has_form = parameters.contains("\"bodyContentType\"")
                    && parameters.contains("form-data");

                if has_body && has_form {
                    return Some(N8nValidationError {
                        error_type: "invalid_value".to_string(),
                        node: Some(node.name.clone()),
                        property: Some("body".to_string()),
                        message: "HTTP Request node cannot have both body and form-data set"
                            .to_string(),
                        severity: "error".to_string(),
                        auto_fix: false,
                        fix_suggestion: Some(
                            "Remove either body or form-data, not both".to_string(),
                        ),
                    });
                }
            }
        }
        None
    }

    fn check_if_branches_connected(wf: &N8nWorkflow) -> Option<N8nValidationError> {
        let if_nodes: Vec<&N8nNode> = wf.nodes.iter().filter(|n| n.type_.contains("If")).collect();

        for if_node in if_nodes {
            let has_true_branch = wf
                .connections
                .iter()
                .any(|c| c.from.node == if_node.name && c.from.index == 1);
            let has_false_branch = wf
                .connections
                .iter()
                .any(|c| c.from.node == if_node.name && c.from.index == 2);

            if !has_true_branch || !has_false_branch {
                return Some(N8nValidationError {
                    error_type: "incomplete_branch".to_string(),
                    node: Some(if_node.name.clone()),
                    property: None,
                    message: format!("IF node '{}' has unconnected branches", if_node.name),
                    severity: "warning".to_string(),
                    auto_fix: true,
                    fix_suggestion: Some(
                        "Connect both true and false branches or add an IF empty result handler"
                            .to_string(),
                    ),
                });
            }
        }
        None
    }

    fn check_required_fields(wf: &N8nWorkflow) -> Option<N8nValidationError> {
        for node in &wf.nodes {
            match node.type_.as_str() {
                "n8n-nodes-base.slack" | "slack" => {
                    if !node.parameters.contains("channel")
                        && !node.parameters.contains("\"channel\"")
                    {
                        return Some(N8nValidationError {
                            error_type: "missing_required".to_string(),
                            node: Some(node.name.clone()),
                            property: Some("channel".to_string()),
                            message: format!(
                                "Slack node '{}' missing required field: channel",
                                node.name
                            ),
                            severity: "error".to_string(),
                            auto_fix: false,
                            fix_suggestion: Some("Add a channel name (e.g., #general)".to_string()),
                        });
                    }
                }
                "n8n-nodes-base.httpRequest" | "HTTP Request" => {
                    if !node.parameters.contains("url") && !node.parameters.contains("\"url\"") {
                        return Some(N8nValidationError {
                            error_type: "missing_required".to_string(),
                            node: Some(node.name.clone()),
                            property: Some("url".to_string()),
                            message: format!(
                                "HTTP Request node '{}' missing required field: url",
                                node.name
                            ),
                            severity: "error".to_string(),
                            auto_fix: false,
                            fix_suggestion: Some("Add a valid URL".to_string()),
                        });
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn check_expression_syntax(wf: &N8nWorkflow) -> Option<N8nValidationError> {
        for node in &wf.nodes {
            let params = &node.parameters;

            if params.contains("{{$json.") && params.contains("}}") {
                let has_malformed = params.contains("{{ ") || params.contains(" }}");
                if has_malformed {
                    return Some(N8nValidationError {
                        error_type: "invalid_expression".to_string(),
                        node: Some(node.name.clone()),
                        property: None,
                        message: format!("Node '{}' has malformed expression syntax", node.name),
                        severity: "error".to_string(),
                        auto_fix: false,
                        fix_suggestion: Some(
                            "Remove extra spaces in expressions like {{ $json.field }}".to_string(),
                        ),
                    });
                }
            }
        }
        None
    }
}

impl Default for N8nValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nWorkflow {
    pub name: String,
    pub nodes: Vec<N8nNode>,
    pub connections: Vec<N8nConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nNode {
    pub id: String,
    pub name: String,
    pub type_: String,
    pub parameters: String,
    pub position: (i32, i32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nConnection {
    pub from: ConnectionPoint,
    pub to: ConnectionPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoint {
    pub node: String,
    pub index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_validation() {
        let validator = N8nValidator::new();

        let workflow = N8nWorkflow {
            name: "test-webhook".to_string(),
            nodes: vec![N8nNode {
                id: "1".to_string(),
                name: "Webhook".to_string(),
                type_: "n8n-nodes-base.webhook".to_string(),
                parameters: "{}".to_string(),
                position: (0, 0),
            }],
            connections: vec![],
        };

        let result = validator.validate(&workflow);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.error_type == "missing_response"));
    }

    #[test]
    fn test_valid_workflow() {
        let validator = N8nValidator::new();

        let workflow = N8nWorkflow {
            name: "test-valid".to_string(),
            nodes: vec![
                N8nNode {
                    id: "1".to_string(),
                    name: "Webhook".to_string(),
                    type_: "n8n-nodes-base.webhook".to_string(),
                    parameters: "{}".to_string(),
                    position: (0, 0),
                },
                N8nNode {
                    id: "2".to_string(),
                    name: "Respond to Webhook".to_string(),
                    type_: "n8n-nodes-base.respondToWebhook".to_string(),
                    parameters: "{}".to_string(),
                    position: (1, 0),
                },
            ],
            connections: vec![N8nConnection {
                from: ConnectionPoint {
                    node: "Webhook".to_string(),
                    index: 0,
                },
                to: ConnectionPoint {
                    node: "Respond to Webhook".to_string(),
                    index: 0,
                },
            }],
        };

        let result = validator.validate(&workflow);
        assert!(result.valid);
    }
}
