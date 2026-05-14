/// P2.2: gRPC service framework — in-memory implementation (no tonic dep yet)
/// Defines service traits matching proto/memflow.proto, with in-process dispatch.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ---- Message types matching proto ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub text: String,
    pub user_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub success: bool,
    pub output: String,
    pub iterations: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub memory_type: String,
    pub importance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<MemoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRequest {
    pub workflow_id: String,
    pub params: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResponse {
    pub success: bool,
    pub result: String,
    pub error: Option<String>,
}

// ---- Service traits ----

#[async_trait]
pub trait AgentService: Send + Sync {
    async fn execute_task(&self, req: &TaskRequest) -> TaskResponse;
}

#[async_trait]
pub trait MemoryService: Send + Sync {
    async fn search(&self, query: &str, k: usize) -> SearchResponse;
}

#[async_trait]
pub trait ExecutorService: Send + Sync {
    async fn execute_workflow(&self, req: &WorkflowRequest) -> WorkflowResponse;
}

// ---- In-process dispatcher (avoids HTTP for same-process calls) ----

pub struct ServiceDispatcher {
    pub agent: Box<dyn AgentService>,
    pub memory: Box<dyn MemoryService>,
    pub executor: Box<dyn ExecutorService>,
}

impl ServiceDispatcher {
    pub fn new(
        agent: Box<dyn AgentService>,
        memory: Box<dyn MemoryService>,
        executor: Box<dyn ExecutorService>,
    ) -> Self {
        Self { agent, memory, executor }
    }

    pub async fn dispatch_task(&self, text: &str, user_id: &str) -> TaskResponse {
        let req = TaskRequest {
            text: text.to_string(),
            user_id: user_id.to_string(),
            session_id: uuid::Uuid::new_v4().to_string(),
        };
        self.agent.execute_task(&req).await
    }

    pub async fn search_memory(&self, query: &str, k: usize) -> SearchResponse {
        self.memory.search(query, k).await
    }

    pub async fn exec_workflow(&self, id: &str, params: &str) -> WorkflowResponse {
        self.executor.execute_workflow(&WorkflowRequest {
            workflow_id: id.to_string(),
            params: params.to_string(),
        }).await
    }
}
