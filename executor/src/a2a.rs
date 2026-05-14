/// T3.1: A2A (Agent-to-Agent) Multi-Agent Collaboration Framework
/// 
/// Extends the basic sub_agent.rs coordinator with proper A2A communication,
/// task decomposition, role-based specialization, and result aggregation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentRole {
    Coordinator,   // Breaks down tasks and assigns to specialists
    Researcher,    // Gathers information, searches, reads
    Executor,      // Executes actions, runs workflows
    Reviewer,      // Validates results, quality check
    Critic,        // Finds issues, suggests improvements
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AAgent {
    pub id: String,
    pub name: String,
    pub role: AgentRole,
    pub model: String,
    pub system_prompt: String,
    pub capabilities: Vec<String>,
    pub max_iterations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATask {
    pub id: String,
    pub parent_id: Option<String>,
    pub description: String,
    pub assigned_role: AgentRole,
    pub context: serde_json::Value,
    pub status: TaskStatus,
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending, Running, Completed, Failed,
}

/// Manages A2A communication and task orchestration
pub struct A2AOrchestrator {
    agents: Arc<RwLock<HashMap<String, A2AAgent>>>,
    task_queue: Arc<RwLock<Vec<A2ATask>>>,
    results: Arc<RwLock<Vec<(String, serde_json::Value)>>>,
    tx: mpsc::Sender<A2ATask>,
    rx: Arc<RwLock<mpsc::Receiver<A2ATask>>>,
}

impl A2AOrchestrator {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1000);
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            task_queue: Arc::new(RwLock::new(Vec::new())),
            results: Arc::new(RwLock::new(Vec::new())),
            tx, rx: Arc::new(RwLock::new(rx)),
        }
    }

    pub async fn register_agent(&self, agent: A2AAgent) {
        self.agents.write().await.insert(agent.id.clone(), agent);
    }

    /// Submit a complex task that will be decomposed and distributed
    pub async fn submit_task(&self, description: &str, context: serde_json::Value) -> String {
        let task_id = uuid::Uuid::new_v4().to_string();
        let task = A2ATask {
            id: task_id.clone(),
            parent_id: None,
            description: description.to_string(),
            assigned_role: AgentRole::Coordinator,
            context,
            status: TaskStatus::Pending,
            result: None,
        };
        self.task_queue.write().await.push(task.clone());
        self.tx.send(task).await.unwrap_or(());
        task_id
    }

    /// Decompose a task into sub-tasks for different agent roles
    pub async fn decompose(&self, task: &A2ATask) -> Vec<A2ATask> {
        let agents = self.agents.read().await;
        let mut sub_tasks = Vec::new();

        // Find available agents by role
        for (id, agent) in agents.iter() {
            if matches!(agent.role, AgentRole::Coordinator) { continue; }
            if !self.is_capable(agent, &task.description) { continue; }

            sub_tasks.push(A2ATask {
                id: uuid::Uuid::new_v4().to_string(),
                parent_id: Some(task.id.clone()),
                description: format!("[{}] {}", agent.role_name(), task.description),
                assigned_role: agent.role.clone(),
                context: task.context.clone(),
                status: TaskStatus::Pending,
                result: None,
            });
        }

        sub_tasks
    }

    fn is_capable(&self, agent: &A2AAgent, task: &str) -> bool {
        let task_lower = task.to_lowercase();
        agent.capabilities.iter().any(|c| task_lower.contains(&c.to_lowercase()))
    }

    /// Collect all results and synthesize a final response
    pub async fn synthesize(&self, parent_id: &str) -> serde_json::Value {
        let results = self.results.read().await;
        let related: Vec<&(String, serde_json::Value)> = results.iter()
            .filter(|(id, _)| id == parent_id)
            .collect();

        serde_json::json!({
            "parent_task": parent_id,
            "sub_results": related.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>(),
            "total_contributions": related.len(),
        })
    }

    pub async fn agent_count(&self) -> usize { self.agents.read().await.len() }
}

impl Default for A2AOrchestrator {
    fn default() -> Self { Self::new() }
}

impl A2AAgent {
    pub fn role_name(&self) -> &'static str {
        match self.role {
            AgentRole::Coordinator => "Coordinator",
            AgentRole::Researcher => "Researcher",
            AgentRole::Executor => "Executor",
            AgentRole::Reviewer => "Reviewer",
            AgentRole::Critic => "Critic",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_registration() {
        let orch = A2AOrchestrator::new();
        orch.register_agent(A2AAgent {
            id: "res_1".into(), name: "Researcher A".into(),
            role: AgentRole::Researcher, model: "gpt-4o".into(),
            system_prompt: "Research and gather info".into(),
            capabilities: vec!["search".into(), "read".into()],
            max_iterations: 5,
        }).await;
        assert_eq!(orch.agent_count().await, 1);
    }

    #[tokio::test]
    async fn test_task_submission() {
        let orch = A2AOrchestrator::new();
        let id = orch.submit_task("Research latest AI trends", serde_json::json!({})).await;
        assert!(!id.is_empty());
    }

    #[test]
    fn test_role_names() {
        assert_eq!(A2AAgent { id: "".into(), name: "".into(), role: AgentRole::Coordinator, model: "".into(), system_prompt: "".into(), capabilities: vec![], max_iterations: 0 }.role_name(), "Coordinator");
        assert_eq!(A2AAgent { id: "".into(), name: "".into(), role: AgentRole::Researcher, model: "".into(), system_prompt: "".into(), capabilities: vec![], max_iterations: 0 }.role_name(), "Researcher");
    }
}



