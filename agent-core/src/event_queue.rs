/// P4.4: Event-driven async queue
/// Decouples agent-core from learning-engine via events

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemFlowEvent {
    TaskCompleted {
        task_id: String,
        workflow_id: String,
        duration_ms: i64,
        success: bool,
    },
    SkillGenerated {
        skill_name: String,
        skill_id: String,
    },
    LearningCycle {
        cycle_id: String,
        patterns_found: usize,
    },
    Error {
        source: String,
        message: String,
    },
}

static EVENT_QUEUE: once_cell::sync::Lazy<Mutex<VecDeque<MemFlowEvent>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(VecDeque::new()));

pub fn emit(event: MemFlowEvent) {
    let mut queue = EVENT_QUEUE.lock().unwrap();
    queue.push_back(event);
    tracing::info!(target: "events", queue_len = %queue.len(), "Event emitted");
}

pub fn poll() -> Option<MemFlowEvent> {
    EVENT_QUEUE.lock().unwrap().pop_front()
}

pub fn poll_all() -> Vec<MemFlowEvent> {
    let mut queue = EVENT_QUEUE.lock().unwrap();
    queue.drain(..).collect()
}

pub fn queue_len() -> usize {
    EVENT_QUEUE.lock().unwrap().len()
}
