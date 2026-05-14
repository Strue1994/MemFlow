use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;
    async fn execute(&self, args: &serde_json::Value) -> ToolResult;
}

pub struct ExecutorTool {
    name: String,
    description: String,
    parameters: serde_json::Value,
    executor: Arc<std::sync::Mutex<executor::Executor>>,
    workflow: Arc<compiler::Workflow>,
}

impl ExecutorTool {
    pub fn new(name: &str, description: &str, executor: Arc<std::sync::Mutex<executor::Executor>>, workflow: Arc<compiler::Workflow>) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters: serde_json::json!({}),
            executor,
            workflow,
        }
    }
}

#[async_trait]
impl Tool for ExecutorTool {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str { &self.description }
    fn parameters(&self) -> serde_json::Value { self.parameters.clone() }

    async fn execute(&self, _args: &serde_json::Value) -> ToolResult {
        let mut exec = self.executor.lock().unwrap();
        match exec.execute(&self.workflow) {
            Ok(val) => ToolResult { success: true, output: val.to_string(), error: None },
            Err(e) => ToolResult { success: false, output: String::new(), error: Some(e.to_string()) },
        }
    }
}

pub struct SimpleTool {
    name: String,
    description: String,
    params: serde_json::Value,
    handler: Box<dyn Fn(&serde_json::Value) -> ToolResult + Send + Sync>,
}

impl SimpleTool {
    pub fn new(name: &str, desc: &str, handler: Box<dyn Fn(&serde_json::Value) -> ToolResult + Send + Sync>) -> Self {
        Self { name: name.to_string(), description: desc.to_string(), params: serde_json::json!({}), handler }
    }
}

#[async_trait]
impl Tool for SimpleTool {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str { &self.description }
    fn parameters(&self) -> serde_json::Value { self.params.clone() }
    async fn execute(&self, args: &serde_json::Value) -> ToolResult { (self.handler)(args) }
}

