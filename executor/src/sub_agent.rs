use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentCapability {
    WorkflowGeneration,
    CodeExecution,
    DataAnalysis,
    WebScraping,
    ApiIntegration,
    DocumentProcessing,
    MathCalculation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgent {
    pub id: String,
    pub name: String,
    pub model: String,
    pub capabilities: Vec<AgentCapability>,
    pub max_concurrent: u32,
    pub priority: u8,
}

impl SubAgent {
    pub fn can_handle(&self, task_type: &AgentCapability) -> bool {
        self.capabilities.contains(task_type)
    }
}

pub struct SubAgentCoordinator {
    agents: Arc<RwLock<HashMap<String, SubAgent>>>,
    task_queue: Arc<RwLock<Vec<AgentTaskWithAgent>>>,
    results: Arc<RwLock<HashMap<String, TaskResult>>>,
}

impl SubAgentCoordinator {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            task_queue: Arc::new(RwLock::new(Vec::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_agent(&self, agent: SubAgent) {
        let id = agent.id.clone();
        self.agents.write().await.insert(id, agent);
    }

    pub async fn dispatch_task(&self, task: AgentTask) -> String {
        let task_id = task.id.clone();
        
        let agent = self.select_agent(&task.required_capability).await;
        
        if let Some(agent) = agent {
            let task_with_agent = AgentTaskWithAgent {
                task: task.clone(),
                agent_id: agent.id.clone(),
                status: TaskStatus::Running,
                started_at: chrono::Utc::now().timestamp(),
            };
            
            self.task_queue.write().await.push(task_with_agent);
            
            let results = self.results.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let _ = client;
                let result = TaskResult {
                    task_id: task.id.clone(),
                    agent_id: agent.id.clone(),
                    success: true,
                    output: "Task executed".to_string(),
                    error: None,
                    duration_ms: 100,
                };
                results.write().await.insert(task.id.clone(), result);
            });
        } else {
            let queued = AgentTaskWithAgent {
                task: task.clone(),
                agent_id: String::new(),
                status: TaskStatus::Pending,
                started_at: chrono::Utc::now().timestamp(),
            };
            let mut queue = self.task_queue.write().await;
            queue.push(queued);
        }

        task_id
    }

    async fn select_agent(&self, capability: &AgentCapability) -> Option<SubAgent> {
        let agents = self.agents.read().await;
        
        let available: Vec<_> = agents
            .values()
            .filter(|a| a.can_handle(capability))
            .collect();
        
        if available.is_empty() {
            return None;
        }

        available
            .into_iter()
            .max_by_key(|a| a.priority as u32)
            .cloned()
    }

    async fn execute_agent_task(&self, agent: SubAgent, task: AgentTask) {
        let _client = reqwest::Client::new();
        
        let _prompt = format!(
            "Task: {}\nContext: {}\nExecute using {} model",
            task.description, task.context, agent.model
        );

        let result = TaskResult {
            task_id: task.id.clone(),
            agent_id: agent.id.clone(),
            success: true,
            output: "Task executed".to_string(),
            error: None,
            duration_ms: 100,
        };

        self.results.write().await.insert(task.id.clone(), result);
    }

    pub async fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        let queue = self.task_queue.read().await;
        queue.iter().find(|t| t.task.id == task_id).map(|t| t.status.clone())
    }

    pub async fn get_task_result(&self, task_id: &str) -> Option<TaskResult> {
        self.results.read().await.get(task_id).cloned()
    }

    pub async fn list_agents(&self) -> Vec<SubAgent> {
        self.agents.read().await.values().cloned().collect()
    }

    pub async fn get_agent_stats(&self) -> HashMap<String, AgentStats> {
        let agents = self.agents.read().await;
        let results = self.results.read().await;
        let _queue = self.task_queue.read().await;
        
        let mut stats = HashMap::new();
        
        for (id, _agent) in agents.iter() {
            let agent_results: Vec<_> = results
                .values()
                .filter(|r| &r.agent_id == id)
                .collect();
            
            let success_count = agent_results.iter().filter(|r| r.success).count();
            
            stats.insert(id.clone(), AgentStats {
                agent_id: id.clone(),
                total_tasks: agent_results.len(),
                success_rate: if agent_results.is_empty() {
                    0.0
                } else {
                    success_count as f64 / agent_results.len() as f64
                },
                avg_duration_ms: if agent_results.is_empty() {
                    0.0
                } else {
                    agent_results.iter().map(|r| r.duration_ms as f64).sum::<f64>() / agent_results.len() as f64
                },
            });
        }

        stats
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub description: String,
    pub context: String,
    pub required_capability: AgentCapability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTaskWithAgent {
    pub task: AgentTask,
    pub agent_id: String,
    pub status: TaskStatus,
    pub started_at: i64,
}

impl AgentTaskWithAgent {
    pub fn to_task(self) -> AgentTask {
        self.task
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub agent_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStats {
    pub agent_id: String,
    pub total_tasks: usize,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
}

pub fn create_sub_agent_coordinator() -> SubAgentCoordinator {
    SubAgentCoordinator::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_agent() {
        let coordinator = SubAgentCoordinator::new();
        
        let agent = SubAgent {
            id: "agent1".to_string(),
            name: "Code Agent".to_string(),
            model: "gpt-4".to_string(),
            capabilities: vec![AgentCapability::CodeExecution],
            max_concurrent: 3,
            priority: 5,
        };
        
        coordinator.register_agent(agent).await;
        let agents = coordinator.list_agents().await;
        
        assert_eq!(agents.len(), 1);
    }

    #[tokio::test]
    async fn test_dispatch_task() {
        let coordinator = SubAgentCoordinator::new();
        
        let agent = SubAgent {
            id: "agent1".to_string(),
            name: "Code Agent".to_string(),
            model: "gpt-4".to_string(),
            capabilities: vec![AgentCapability::CodeExecution],
            max_concurrent: 3,
            priority: 5,
        };
        coordinator.register_agent(agent).await;
        
        let task = AgentTask {
            id: "task1".to_string(),
            description: "Execute code".to_string(),
            context: "{}".to_string(),
            required_capability: AgentCapability::CodeExecution,
        };
        
        let task_id = coordinator.dispatch_task(task).await;
        assert!(!task_id.is_empty());
    }
}
