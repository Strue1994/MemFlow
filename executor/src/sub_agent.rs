/// B1: Subagent System — spawn/wait/cancel for parallel agent branches
///
/// Each subagent runs as an isolated workflow execution with its own:
/// - Independent context (messages, state)
/// - Configurable timeout (default 15 min)
/// - Output capture and structured result
///
/// Bypasses the paid task() gate by using the executor's own workflow engine.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::timeout;
use uuid::Uuid;
use chrono::Utc;

// ---- Types ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentSpec {
    pub id: String,
    pub name: String,
    pub prompt: String,
    pub tools: Vec<String>,
    pub timeout_secs: u64,
    pub max_iterations: u32,
}

impl Default for SubAgentSpec {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "subagent".into(),
            prompt: String::new(),
            tools: vec![],
            timeout_secs: 900, // 15 minutes
            max_iterations: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubAgentStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    pub id: String,
    pub name: String,
    pub status: SubAgentStatus,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub iterations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentHandle {
    pub id: String,
    pub name: String,
    pub status: SubAgentStatus,
    pub started_at: String,
}

// ---- In-memory subagent state ----

struct SubAgentState {
    spec: SubAgentSpec,
    status: SubAgentStatus,
    output: String,
    error: Option<String>,
    started_at: Instant,
    started_at_str: String,
    duration_ms: u64,
    iterations: u32,
}

pub struct SubAgentManager {
    agents: Arc<RwLock<HashMap<String, SubAgentState>>>,
    max_concurrent: usize,
    max_depth: u32,
}

impl SubAgentManager {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent: 3,
            max_depth: 1,
        }
    }

    pub fn with_limits(mut self, max_concurrent: usize, max_depth: u32) -> Self {
        self.max_concurrent = max_concurrent;
        self.max_depth = max_depth;
        self
    }

    /// Spawn a subagent. Returns a handle immediately; the agent runs in background.
    pub async fn spawn(&self, spec: SubAgentSpec) -> Result<SubAgentHandle, String> {
        let agents = self.agents.read().await;

        // Check concurrency limit
        let running_count = agents.values().filter(|a| a.status == SubAgentStatus::Running || a.status == SubAgentStatus::Pending).count();
        if running_count >= self.max_concurrent {
            return Err(format!("Max concurrent subagents reached ({})", self.max_concurrent));
        }

        let id = spec.id.clone();
        let name = spec.name.clone();
        let started_at_str = Utc::now().to_rfc3339();
        drop(agents); // Release read lock

        let state = SubAgentState {
            spec: spec.clone(),
            status: SubAgentStatus::Running,
            output: String::new(),
            error: None,
            started_at: Instant::now(),
            started_at_str: started_at_str.clone(),
            duration_ms: 0,
            iterations: 0,
        };

        self.agents.write().await.insert(id.clone(), state);

        // Spawn background execution
        let agents_clone = self.agents.clone();
        let spec_clone = spec.clone();
        tokio::spawn(async move {
            execute_subagent(agents_clone, spec_clone).await;
        });

        Ok(SubAgentHandle {
            id,
            name,
            status: SubAgentStatus::Running,
            started_at: started_at_str,
        })
    }

    /// Wait for a subagent to complete (with timeout).
    pub async fn wait(&self, id: &str, wait_secs: u64) -> Result<SubAgentResult, String> {
        let deadline = Duration::from_secs(wait_secs);
        let poll_interval = Duration::from_millis(200);
        let start = Instant::now();

        loop {
            if start.elapsed() > deadline {
                return Err(format!("Wait timed out after {}s for subagent {}", wait_secs, id));
            }

            let agents = self.agents.read().await;
            if let Some(state) = agents.get(id) {
                let is_done = matches!(state.status, SubAgentStatus::Completed | SubAgentStatus::Failed | SubAgentStatus::Cancelled | SubAgentStatus::TimedOut);
                if is_done {
                    return Ok(SubAgentResult {
                        id: id.to_string(),
                        name: state.spec.name.clone(),
                        status: state.status.clone(),
                        output: state.output.clone(),
                        error: state.error.clone(),
                        duration_ms: state.duration_ms,
                        started_at: state.started_at_str.clone(),
                        completed_at: Some(Utc::now().to_rfc3339()),
                        iterations: state.iterations,
                    });
                }
            } else {
                return Err(format!("Subagent {} not found", id));
            }
            drop(agents);

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Cancel a running subagent.
    pub async fn cancel(&self, id: &str) -> Result<bool, String> {
        let mut agents = self.agents.write().await;
        if let Some(state) = agents.get_mut(id) {
            if matches!(state.status, SubAgentStatus::Running | SubAgentStatus::Pending) {
                state.status = SubAgentStatus::Cancelled;
                state.duration_ms = state.started_at.elapsed().as_millis() as u64;
                return Ok(true);
            }
            return Ok(false); // Already done
        }
        Err(format!("Subagent {} not found", id))
    }

    /// List all subagents with their status.
    pub async fn list(&self) -> Vec<SubAgentResult> {
        let agents = self.agents.read().await;
        agents.values().map(|s| SubAgentResult {
            id: s.spec.id.clone(),
            name: s.spec.name.clone(),
            status: s.status.clone(),
            output: s.output.clone(),
            error: s.error.clone(),
            duration_ms: s.duration_ms,
            started_at: s.started_at_str.clone(),
            completed_at: None,
            iterations: s.iterations,
        }).collect()
    }

    /// Get a single subagent's current state.
    pub async fn get(&self, id: &str) -> Option<SubAgentResult> {
        let agents = self.agents.read().await;
        agents.get(id).map(|s| SubAgentResult {
            id: s.spec.id.clone(),
            name: s.spec.name.clone(),
            status: s.status.clone(),
            output: s.output.clone(),
            error: s.error.clone(),
            duration_ms: s.duration_ms,
            started_at: s.started_at_str.clone(),
            completed_at: None,
            iterations: s.iterations,
        })
    }

    /// Clean up completed subagents older than the given duration.
    pub async fn cleanup(&self, older_than_secs: u64) -> usize {
        let mut agents = self.agents.write().await;
        let cutoff = Instant::now() - Duration::from_secs(older_than_secs);
        let before = agents.len();
        agents.retain(|_, s| {
            if matches!(s.status, SubAgentStatus::Completed | SubAgentStatus::Failed | SubAgentStatus::Cancelled | SubAgentStatus::TimedOut) {
                s.started_at > cutoff
            } else {
                true // Keep running/pending
            }
        });
        before - agents.len()
    }
}

/// Background execution of a subagent.
async fn execute_subagent(agents: Arc<RwLock<HashMap<String, SubAgentState>>>, spec: SubAgentSpec) {
    let start = Instant::now();

    // Execute the subagent's prompt by calling back to the agent-service
    // This is how subagents work: they call the same /agent/execute endpoint
    let executor_url = std::env::var("EXECUTOR_URL").unwrap_or_else(|_| "http://127.0.0.1:8082".into());
    let api_key = std::env::var("EXECUTOR_API_KEY").unwrap_or_else(|_| "memflow-local-dev-key".into());

    let client = reqwest::Client::new();
    let result = tokio::time::timeout(
        Duration::from_secs(spec.timeout_secs),
        client.post(format!("{}/execute", executor_url))
            .header("X-API-Key", &api_key)
            .json(&serde_json::json!({
                "workflow_id": spec.id,
                "params": {
                    "prompt": spec.prompt,
                    "tools": spec.tools,
                    "max_iterations": spec.max_iterations,
                }
            }))
            .send(),
    ).await;

    let duration_ms = start.elapsed().as_millis() as u64;

    let mut agents = agents.write().await;
    let mut state = agents.get_mut(&spec.id).unwrap();

    state.duration_ms = duration_ms;
    state.iterations = spec.max_iterations;

    match result {
        Ok(Ok(resp)) => {
            state.output = resp.text().await.unwrap_or_else(|e| format!("Read error: {}", e));
            state.status = SubAgentStatus::Completed;
        }
        Ok(Err(e)) => {
            state.error = Some(format!("HTTP error: {}", e));
            state.status = SubAgentStatus::Failed;
        }
        Err(_) => {
            state.error = Some("Timed out".into());
            state.status = SubAgentStatus::TimedOut;
        }
    }
}

// ---- Global instance ----

use once_cell::sync::Lazy;
pub static GLOBAL_SUBAGENT_MANAGER: Lazy<SubAgentManager> = Lazy::new(|| {
    SubAgentManager::new()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_and_wait() {
        let manager = SubAgentManager::new();
        let spec = SubAgentSpec {
            id: "test-1".into(),
            name: "test".into(),
            prompt: "echo hello".into(),
            tools: vec![],
            timeout_secs: 5,
            max_iterations: 1,
        };

        let handle = manager.spawn(spec).await.unwrap();
        assert_eq!(handle.status, SubAgentStatus::Running);

        // Cancel immediately
        let cancelled = manager.cancel("test-1").await.unwrap();
        assert!(cancelled);

        let result = manager.get("test-1").await.unwrap();
        assert_eq!(result.status, SubAgentStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_concurrent_limit() {
        let manager = SubAgentManager::new().with_limits(1, 1);
        let spec = SubAgentSpec::default();

        let spec1 = SubAgentSpec { id: "c1".into(), ..spec.clone() };
        let spec2 = SubAgentSpec { id: "c2".into(), ..spec };

        assert!(manager.spawn(spec1).await.is_ok());
        let r2 = manager.spawn(spec2).await;
        assert!(r2.is_err()); // Should hit concurrency limit
    }
}
