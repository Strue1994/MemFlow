use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum YamlError {
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Conversion error: {0}")]
    Conversion(String),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct YamlWorkflow {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub steps: Vec<YamlStep>,
    #[serde(default)]
    pub conditions: Option<YamlConditions>,
    #[serde(default)]
    pub return_var: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct YamlStep {
    pub id: String,
    #[serde(flatten)]
    pub action: YamlAction,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum YamlAction {
    Http {
        method: String,
        url: String,
        #[serde(default)]
        headers: serde_json::Map<String, serde_json::Value>,
        #[serde(default)]
        body: Option<serde_json::Value>,
    },
    Set {
        value: serde_json::Value,
    },
    If {
        condition: String,
        then: Vec<YamlStep>,
        #[serde(default)]
        else_: Option<Vec<YamlStep>>,
    },
    For {
        #[serde(default)]
        over: Option<String>,
        #[serde(default)]
        in_: Option<String>,
        #[serde(default)]
        from: Option<i64>,
        #[serde(default)]
        to: Option<i64>,
        do_: Vec<YamlStep>,
    },
    Call {
        workflow: String,
        #[serde(default)]
        params: Option<serde_json::Value>,
    },
    Code {
        script: String,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct YamlConditions {
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub retry: Option<YamlRetry>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct YamlRetry {
    pub count: u32,
    #[serde(default)]
    pub delay: Option<u64>,
}

pub struct YamlParser;

impl YamlParser {
    pub fn parse(yaml_str: &str) -> Result<YamlWorkflow, YamlError> {
        serde_yaml::from_str(yaml_str).map_err(|e| YamlError::Parse(e.to_string()))
    }

    pub fn to_n8n_json(workflow: &YamlWorkflow) -> Result<serde_json::Value, YamlError> {
        let n8n_nodes: Vec<serde_json::Value> = workflow
            .steps
            .iter()
            .enumerate()
            .map(|(index, step)| {
                let node_type = Self::get_node_type(&step.action);
                let (parameters, inputs) =
                    Self::get_node_params(&step.action, index, &workflow.steps);

                serde_json::json!({
                    "id": step.id,
                    "name": step.id,
                    "type": node_type,
                    "position": [(index as i32) * 300, 100],
                    "parameters": parameters,
                    "inputs": inputs,
                })
            })
            .collect();

        let connections = Self::build_connections(&workflow.steps);

        Ok(serde_json::json!({
            "name": workflow.name,
            "nodes": n8n_nodes,
            "connections": connections,
        }))
    }

    fn get_node_type(action: &YamlAction) -> &str {
        match action {
            YamlAction::Http { .. } => "n8n-nodes-base.httpRequest",
            YamlAction::Set { .. } => "n8n-nodes-base.set",
            YamlAction::If { .. } => "n8n-nodes-base.if",
            YamlAction::For { .. } => "n8n-nodes-base.splitInBatches",
            YamlAction::Call { .. } => "n8n-nodes-base.executeWorkflowTrigger",
            YamlAction::Code { .. } => "n8n-nodes-base.code",
        }
    }

    fn get_node_params(
        action: &YamlAction,
        index: usize,
        _steps: &[YamlStep],
    ) -> (serde_json::Value, Vec<String>) {
        match action {
            YamlAction::Http {
                method,
                url,
                headers,
                body,
            } => (
                serde_json::json!({
                    "method": method,
                    "url": url,
                    "sendHeaders": true,
                    "headers": headers,
                    "body": body,
                }),
                vec![],
            ),
            YamlAction::Set { value } => (
                serde_json::json!({
                    "value": value,
                }),
                vec![],
            ),
            YamlAction::If { condition, .. } => (
                serde_json::json!({
                    "conditions": [{"leftOperand": [], "operator": "equals", "rightOperand": ""}],
                    "value1": condition,
                }),
                vec![],
            ),
            _ => (serde_json::json!({}), vec![]),
        }
    }

    fn build_connections(steps: &[YamlStep]) -> Vec<serde_json::Value> {
        let mut connections = Vec::new();

        for (i, step) in steps.iter().enumerate() {
            if i < steps.len() - 1 {
                let next_id = &steps[i + 1].id;
                connections.push(serde_json::json!({
                    "from": step.id,
                    "to": next_id,
                }));
            }
        }

        connections
    }

    pub fn from_n8n_json(n8n_json: &serde_json::Value) -> Result<YamlWorkflow, YamlError> {
        let name = n8n_json
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unnamed Workflow")
            .to_string();

        let nodes = n8n_json
            .get("nodes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| YamlError::Conversion("Missing nodes array".to_string()))?;

        let mut steps = Vec::new();
        for node in nodes {
            let id = node
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("step")
                .to_string();
            let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let params = node.get("parameters").cloned().unwrap_or_default();

            let action = if node_type.contains("httpRequest") {
                YamlAction::Http {
                    method: params
                        .get("method")
                        .and_then(|v| v.as_str())
                        .unwrap_or("GET")
                        .to_string(),
                    url: params
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    headers: params
                        .get("headers")
                        .and_then(|v| v.as_object())
                        .cloned()
                        .unwrap_or_default(),
                    body: params.get("body").cloned(),
                }
            } else if node_type.contains("set") {
                YamlAction::Set {
                    value: params.get("value").cloned().unwrap_or_default(),
                }
            } else if node_type.contains("code") {
                YamlAction::Code {
                    script: params
                        .get("jsCode")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                }
            } else {
                YamlAction::Set {
                    value: serde_json::Value::Null,
                }
            };

            steps.push(YamlStep { id, action });
        }

        Ok(YamlWorkflow {
            name,
            description: None,
            steps,
            conditions: None,
            return_var: None,
        })
    }
}

pub fn yaml_to_n8n(yaml_str: &str) -> Result<String, YamlError> {
    let workflow = YamlParser::parse(yaml_str)?;
    let n8n_json = YamlParser::to_n8n_json(&workflow)?;
    serde_json::to_string_pretty(&n8n_json).map_err(|e| YamlError::Conversion(e.to_string()))
}

pub fn n8n_to_yaml(n8n_json_str: &str) -> Result<String, YamlError> {
    let n8n_json: serde_json::Value =
        serde_json::from_str(n8n_json_str).map_err(|e| YamlError::Parse(e.to_string()))?;
    let workflow = YamlParser::from_n8n_json(&n8n_json)?;
    serde_yaml::to_string(&workflow).map_err(|e| YamlError::Conversion(e.to_string()))
}
