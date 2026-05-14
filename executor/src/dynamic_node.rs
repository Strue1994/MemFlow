use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicNode {
    pub node_type: String,
    pub category: NodeCategory,
    pub parameters_schema: HashMap<String, ParameterSchema>,
    pub executor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

    pub fn load_from_directory(&mut self, dir_path: &str) -> Result<u32, String> {
        let path = Path::new(dir_path);
        if !path.exists() {
            return Err(format!("Directory not found: {}", dir_path));
        }

        let mut loaded = 0u32;
        
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(node_def) = serde_json::from_str::<NodeDefinition>(&content) {
                            let dynamic_node = node_def.to_dynamic_node();
                            self.register_node(dynamic_node);
                            loaded += 1;
                        }
                    }
                }
            }
        }
        
        Ok(loaded)
    }

    pub fn register_from_json(&mut self, json: &str) -> Result<(), String> {
        let node_def: NodeDefinition = serde_json::from_str(json)
            .map_err(|e| format!("Invalid node JSON: {}", e))?;
        
        let node = node_def.to_dynamic_node();
        self.register_node(node);
        Ok(())
    }
}

impl Default for DynamicNodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDefinition {
    pub name: String,
    pub description: Option<String>,
    pub category: NodeCategory,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub executor: ExecutorConfig,
}

impl NodeDefinition {
    pub fn to_dynamic_node(self) -> DynamicNode {
        let mut parameters_schema = HashMap::new();
        
        if let Some(input) = self.input_schema {
            if let Some(props) = input.get("properties").and_then(|p| p.as_object()) {
                for (name, schema) in props {
                    let param_type = schema.get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("string")
                        .to_string();
                    let required = input.get("required")
                        .and_then(|r| r.as_array())
                        .map(|arr| arr.iter().any(|v| v.as_str() == Some(name)))
                        .unwrap_or(false);
                    
                    parameters_schema.insert(name.clone(), ParameterSchema {
                        name: name.clone(),
                        param_type,
                        required,
                        default: schema.get("default").cloned(),
                        description: schema.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
                    });
                }
            }
        }

        DynamicNode {
            node_type: self.name.clone(),
            category: self.category,
            parameters_schema,
            executor: Some(self.executor.executor_type),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    #[serde(default)]
    pub executor_type: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub wasm_module: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub inline_js: Option<String>,
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
            "url": "https://example.com",
            "method": "GET"
        });
        assert!(validate_node_parameters("n8n-nodes-base.httpRequest", &params).is_ok());
    }
}


