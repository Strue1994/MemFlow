use crate::error::ParseError;
use crate::ir::HttpMethod;
use crate::ir::Instruction;
pub use crate::ir::Workflow;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct N8nNode {
    #[serde(rename = "type")]
    node_type: String,
    parameters: N8nParameters,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum N8nParameters {
    HttpRequest {
        url: String,
        #[serde(default)]
        headers: Vec<(String, String)>,
        #[serde(default = "default_response_format")]
        response_format: String,
        #[serde(default)]
        method: Option<String>,
        #[serde(default)]
        body: Option<Value>,
        #[serde(default)]
        timeout: Option<u64>,
        #[serde(default)]
        retries: Option<u32>,
        #[serde(default)]
        send_headers: Option<bool>,
        #[serde(default)]
        send_body: Option<bool>,
    },
    Set {
        #[serde(rename = "values")]
        values: Value,
    },
    Code {
        #[serde(default, rename = "jsCode")]
        js_code: Option<String>,
        #[serde(default)]
        output: Option<String>,
    },
    DBQuery {
        #[serde(default)]
        connection: Option<String>,
        #[serde(default)]
        query: Option<String>,
        #[serde(default)]
        params: Option<Value>,
    },
    ReadFile {
        #[serde(default)]
        path: Option<String>,
    },
    WriteFile {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        content: Option<Value>,
        #[serde(default)]
        append: Option<bool>,
    },
    SendEmail {
        #[serde(default)]
        to: Option<String>,
        #[serde(default)]
        subject: Option<String>,
        #[serde(default)]
        body: Option<String>,
        #[serde(default)]
        smtp_config: Option<String>,
    },
    CallWasm {
        #[serde(default)]
        module_id: Option<String>,
        #[serde(default)]
        function: Option<String>,
        #[serde(default)]
        params: Option<Value>,
    },
    Generic(serde_json::Map<String, Value>),
}

fn default_response_format() -> String {
    "response".to_string()
}

pub fn parse_n8n_workflow(json_str: &str) -> Result<Workflow, ParseError> {
    let json: serde_json::Value = serde_json::from_str(json_str)?;
    let nodes = json
        .get("nodes")
        .ok_or_else(|| ParseError::MissingField("nodes".to_string()))?
        .as_array()
        .ok_or_else(|| ParseError::MissingField("nodes array".to_string()))?;

    let mut instructions = Vec::new();

    for node in nodes {
        let node: N8nNode = serde_json::from_value(node.clone())
            .map_err(|_| ParseError::UnsupportedNodeType("invalid node".to_string()))?;

        match node.node_type.as_str() {
            "n8n-nodes-base.manualTrigger" | "manualTrigger" | "trigger" => {
                // Trigger nodes mark workflow entry in the editor but do not emit IR instructions.
            }
            "n8n-nodes-base.httpRequest" | "httpRequest" => {
                if let N8nParameters::HttpRequest {
                    url,
                    headers,
                    response_format,
                    method,
                    body,
                    timeout,
                    retries,
                    ..
                } = node.parameters
                {
                    let http_method = match method.as_deref() {
                        Some("POST") => HttpMethod::Post,
                        Some("PUT") => HttpMethod::Put,
                        Some("DELETE") => HttpMethod::Delete,
                        Some("PATCH") => HttpMethod::Patch,
                        _ => HttpMethod::Get,
                    };
                    instructions.push(Instruction::HttpRequest {
                        method: http_method,
                        url,
                        headers,
                        body,
                        timeout_ms: timeout,
                        max_retries: retries,
                        output_var: response_format,
                    });
                }
            }
            "n8n-nodes-base.set" | "set" => {
                if let N8nParameters::Set { values } = node.parameters {
                    if let Some(obj) = values.as_object() {
                        for (key, val) in obj {
                            instructions.push(Instruction::SetVariable {
                                name: key.clone(),
                                value: val.clone(),
                            });
                        }
                    }
                }
            }
            "n8n-nodes-base.code" | "code" => {
                return Err(ParseError::UnsupportedNodeType(
                    "n8n-nodes-base.code (code execution not supported yet)".to_string(),
                ));
            }
            "n8n-nodes-base.postgres" | "postgres" | "n8n-nodes-base.sqlite" | "sqlite" => {
                if let N8nParameters::DBQuery {
                    connection,
                    query,
                    params,
                } = node.parameters
                {
                    let conn = connection.unwrap_or_else(|| "default".to_string());
                    let q = query.unwrap_or_default();
                    let p = params
                        .map(|v| {
                            if let Some(arr) = v.as_array() {
                                arr.clone()
                            } else {
                                vec![v]
                            }
                        })
                        .unwrap_or_default();
                    instructions.push(Instruction::DBQuery {
                        connection: conn,
                        query: q,
                        params: p,
                        output_var: "result".to_string(),
                    });
                }
            }
            "n8n-nodes-base.readBinaryFile" | "readFile" => {
                if let N8nParameters::ReadFile { path } = node.parameters {
                    if let Some(p) = path {
                        instructions.push(Instruction::ReadFile {
                            path: p,
                            output_var: "content".to_string(),
                        });
                    }
                }
            }
            "n8n-nodes-base.writeBinaryFile" | "writeFile" => {
                if let N8nParameters::WriteFile {
                    path,
                    content,
                    append,
                } = node.parameters
                {
                    if let Some(p) = path {
                        instructions.push(Instruction::WriteFile {
                            path: p,
                            content: content.unwrap_or(Value::String(String::new())),
                            append: append.unwrap_or(false),
                        });
                    }
                }
            }
            "n8n-nodes-base.emailSend" | "emailSend" | "email" => {
                if let N8nParameters::SendEmail {
                    to,
                    subject,
                    body,
                    smtp_config,
                } = node.parameters
                {
                    instructions.push(Instruction::SendEmail {
                        to: to.unwrap_or_default(),
                        subject: subject.unwrap_or_default(),
                        body: body.unwrap_or_default(),
                        smtp_config: smtp_config.unwrap_or_else(|| "default".to_string()),
                    });
                }
            }
            "wasm" | "n8n-nodes-base.wasm" => {
                if let N8nParameters::CallWasm {
                    module_id,
                    function,
                    params,
                } = node.parameters
                {
                    instructions.push(Instruction::CallWasm {
                        module_id: module_id.unwrap_or_default(),
                        function: function.unwrap_or_default(),
                        params: params
                            .map(|v| {
                                if let Some(arr) = v.as_array() {
                                    arr.clone()
                                } else {
                                    vec![v]
                                }
                            })
                            .unwrap_or_default(),
                        output_var: "result".to_string(),
                    });
                }
            }
            _ => {
                return Err(ParseError::UnsupportedNodeType(node.node_type));
            }
        }
    }

    Ok(Workflow::from_instructions(instructions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_and_set() {
        let json = r#"{
          "nodes": [
            {
              "id": "0",
              "type": "trigger",
              "parameters": {}
            },
            {
              "id": "1",
              "type": "n8n-nodes-base.httpRequest",
              "parameters": { "url": "https://api.github.com/zen" }
            },
            {
              "id": "2",
              "type": "n8n-nodes-base.set",
              "parameters": { "values": { "myVar": "hello" } }
            }
          ]
        }"#;

        let workflow = parse_n8n_workflow(json).unwrap();
        assert_eq!(
            workflow
                .nodes
                .first()
                .map(|n| n.instructions.len())
                .unwrap_or(0),
            2
        );

        let wf = workflow.nodes.first().unwrap();
        match &wf.instructions[0] {
            Instruction::HttpRequest {
                url, output_var, ..
            } => {
                assert_eq!(url, "https://api.github.com/zen");
                assert_eq!(output_var, "response");
            }
            _ => panic!("Expected HttpRequest"),
        }

        match &wf.instructions[1] {
            Instruction::SetVariable { name, value } => {
                assert_eq!(name, "myVar");
                assert_eq!(value, "hello");
            }
            _ => panic!("Expected SetVariable"),
        }
    }

    #[test]
    fn test_code_node_is_rejected() {
        let json = r#"{
          "nodes": [
            {
              "id": "1",
              "type": "n8n-nodes-base.code",
              "parameters": { "jsCode": "return 1;" }
            }
          ]
        }"#;

        let error = parse_n8n_workflow(json).unwrap_err();
        assert!(
            error.to_string().contains("n8n-nodes-base.code"),
            "unexpected error: {error}"
        );
    }
}
