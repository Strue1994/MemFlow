use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicNode {
    pub node_type: String,
    pub category: NodeCategory,
    pub parameters_schema: HashMap<String, ParameterSchema>,
    pub executor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeCategory {
    Trigger,
    Action,
    Logic,
    Network,
    Data,
    AI,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSchema {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub description: Option<String>,
}

pub struct DynamicNodeRegistry {
    nodes: HashMap<String, DynamicNode>,
}

impl DynamicNodeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            nodes: HashMap::new(),
        };
        registry.register_builtin_nodes();
        registry
    }

    fn register_builtin_nodes(&mut self) {
        self.register_node(DynamicNode {
            node_type: "n8n-nodes-base.httpRequest".to_string(),
            category: NodeCategory::Network,
            parameters_schema: HashMap::from([
                (
                    "url".to_string(),
                    ParameterSchema {
                        name: "URL".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        default: None,
                        description: Some("The URL to request".to_string()),
                    },
                ),
                (
                    "method".to_string(),
                    ParameterSchema {
                        name: "Method".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        default: Some(serde_json::json!("GET")),
                        description: Some("HTTP method".to_string()),
                    },
                ),
            ]),
            executor: Some("http".to_string()),
        });

        self.register_node(DynamicNode {
            node_type: "n8n-nodes-base.code".to_string(),
            category: NodeCategory::Data,
            parameters_schema: HashMap::from([(
                "jsCode".to_string(),
                ParameterSchema {
                    name: "JavaScript Code".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    default: None,
                    description: Some("JavaScript code to execute".to_string()),
                },
            )]),
            executor: Some("code".to_string()),
        });

        self.register_node(DynamicNode {
            node_type: "n8n-nodes-base.if".to_string(),
            category: NodeCategory::Logic,
            parameters_schema: HashMap::from([(
                "conditions".to_string(),
                ParameterSchema {
                    name: "Conditions".to_string(),
                    param_type: "object".to_string(),
                    required: true,
                    default: None,
                    description: Some("Conditions to evaluate".to_string()),
                },
            )]),
            executor: Some("if".to_string()),
        });
    }

    fn register_node(&mut self, node: DynamicNode) {
        self.nodes.insert(node.node_type.clone(), node);
    }

    pub fn get_node(&self, node_type: &str) -> Option<&DynamicNode> {
        self.nodes.get(node_type)
    }

    pub fn list_nodes(&self) -> Vec<&DynamicNode> {
        self.nodes.values().collect()
    }

    pub fn list_nodes_by_category(&self, category: &NodeCategory) -> Vec<&DynamicNode> {
        self.nodes
            .values()
            .filter(|n| &n.category == category)
            .collect()
    }
}

impl Default for DynamicNodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn validate_node_parameters(node_type: &str, params: &serde_json::Value) -> Result<(), String> {
    let registry = DynamicNodeRegistry::new();

    if let Some(node) = registry.get_node(node_type) {
        for (param_name, schema) in &node.parameters_schema {
            if schema.required {
                if !params.get(param_name).is_some()
                    && !params
                        .get("parameters")
                        .and_then(|p| p.get(param_name))
                        .is_some()
                {
                    return Err(format!("Missing required parameter: {}", param_name));
                }
            }
        }
        Ok(())
    } else {
        Err(format!("Unknown node type: {}", node_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry() {
        let registry = DynamicNodeRegistry::new();
        assert!(registry.get_node("n8n-nodes-base.httpRequest").is_some());
    }

    #[test]
    fn test_validate_params() {
        let params = serde_json::json!({
            "url": "https://example.com"
        });
        assert!(validate_node_parameters("n8n-nodes-base.httpRequest", &params).is_ok());
    }
}
