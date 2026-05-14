use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RLAction {
    Promote,
    Rollback,
    AdjustTimeout,
    AdjustRetry,
    DoNothing,
}

impl RLAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            RLAction::Promote => "promote",
            RLAction::Rollback => "rollback",
            RLAction::AdjustTimeout => "adjust_timeout",
            RLAction::AdjustRetry => "adjust_retry",
            RLAction::DoNothing => "do_nothing",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "promote" => Some(RLAction::Promote),
            "rollback" => Some(RLAction::Rollback),
            "adjust_timeout" => Some(RLAction::AdjustTimeout),
            "adjust_retry" => Some(RLAction::AdjustRetry),
            "do_nothing" => Some(RLAction::DoNothing),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLState {
    pub execution_history: VecDeque<ExecutionMetrics>,
    pub history_optimization_count: u32,
    pub current_version_age_days: u32,
    pub current_timeout_ms: u32,
    pub current_retry_count: u32,
}

impl Default for RLState {
    fn default() -> Self {
        Self {
            execution_history: VecDeque::new(),
            history_optimization_count: 0,
            current_version_age_days: 0,
            current_timeout_ms: 30000,
            current_retry_count: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub success_rate: f64,
    pub latency_ms: f64,
    pub token_consumption: u32,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLReward {
    pub success_change: f64,
    pub latency_change: f64,
    pub token_change: f64,
    pub total: f64,
}

impl RLReward {
    pub fn calculate(before: &ExecutionMetrics, after: &ExecutionMetrics) -> Self {
        let success_change = after.success_rate - before.success_rate;
        let latency_change = if before.latency_ms > 0.0 {
            (before.latency_ms - after.latency_ms) / before.latency_ms
        } else {
            0.0
        };
        let token_change = if before.token_consumption > 0 {
            -(after.token_consumption as f64 - before.token_consumption as f64) 
                / before.token_consumption as f64
        } else {
            0.0
        };

        let total = success_change * 1.0 + latency_change * 0.5 - token_change * 0.2;

        Self {
            success_change,
            latency_change,
            token_change,
            total,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLDecision {
    pub action: RLAction,
    pub confidence: f32,
    pub reasoning: String,
    pub state_snapshot: RLState,
    pub timestamp: i64,
}

pub struct RLDecisionMaker {
    model_endpoint: String,
    fallback_enabled: bool,
    use_rule_engine_on_failure: bool,
    state_history: Arc<RwLock<Vec<RLState>>>,
    decision_history: Arc<RwLock<Vec<RLDecision>>>,
}

impl RLDecisionMaker {
    pub fn new(model_endpoint: String) -> Self {
        Self {
            model_endpoint,
            fallback_enabled: true,
            use_rule_engine_on_failure: true,
            state_history: Arc::new(RwLock::new(Vec::new())),
            decision_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_fallback(mut self, enabled: bool) -> Self {
        self.fallback_enabled = enabled;
        self
    }

    pub fn with_rule_fallback(mut self, enabled: bool) -> Self {
        self.use_rule_engine_on_failure = enabled;
        self
    }

    pub async fn decide(&self, state: RLState) -> RLDecision {
        let state_vector = self.state_to_vector(&state);
        
        let result = self.call_rl_model(&state_vector).await;

        let decision = match result {
            Ok(response) => {
                let action = RLAction::from_str(&response.action).unwrap_or(RLAction::DoNothing);
                RLDecision {
                    action,
                    confidence: response.confidence,
                    reasoning: response.reasoning,
                    state_snapshot: state.clone(),
                    timestamp: chrono::Utc::now().timestamp(),
                }
            }
            Err(e) => {
                if self.use_rule_engine_on_failure {
                    self.rule_based_decision(state.clone()).await
                } else {
                    RLDecision {
                        action: RLAction::DoNothing,
                        confidence: 0.0,
                        reasoning: format!("RL model unavailable: {}", e),
                        state_snapshot: state.clone(),
                        timestamp: chrono::Utc::now().timestamp(),
                    }
                }
            }
        };

        let mut history = self.decision_history.write().await;
        history.push(decision.clone());
        
        let mut states = self.state_history.write().await;
        states.push(state);

        if history.len() > 1000 {
            history.drain(0..500);
            states.drain(0..500);
        }

        decision
    }

    async fn rule_based_decision(&self, state: RLState) -> RLDecision {
        let recent = state.execution_history.iter().take(10).collect::<Vec<_>>();
        
        if recent.is_empty() {
            return RLDecision {
                action: RLAction::DoNothing,
                confidence: 0.5,
                reasoning: "No execution history".to_string(),
                state_snapshot: state,
                timestamp: chrono::Utc::now().timestamp(),
            };
        }

        let avg_success: f64 = recent.iter().map(|m| m.success_rate).sum::<f64>() / recent.len() as f64;
        let avg_latency: f64 = recent.iter().map(|m| m.latency_ms).sum::<f64>() / recent.len() as f64;
        let avg_token: f64 = recent.iter().map(|m| m.token_consumption as f64).sum::<f64>() / recent.len() as f64;

        let action = if avg_success < 0.7 {
            RLAction::Rollback
        } else if avg_success > 0.95 && avg_latency > 1000.0 {
            RLAction::AdjustTimeout
        } else if avg_success > 0.9 && state.current_retry_count < 2 {
            RLAction::AdjustRetry
        } else if avg_success > 0.85 && state.history_optimization_count < 5 {
            RLAction::Promote
        } else {
            RLAction::DoNothing
        };

        RLDecision {
            action,
            confidence: 0.7,
            reasoning: format!(
                "Rule-based: success={:.2}, latency={:.0}, tokens={:.0}",
                avg_success, avg_latency, avg_token
            ),
            state_snapshot: state,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    fn state_to_vector(&self, state: &RLState) -> Vec<f32> {
        let mut vector = Vec::with_capacity(35);

        let recent: Vec<_> = state.execution_history.iter().take(10).cloned().collect();
        
        for i in 0..10 {
            if let Some(m) = recent.get(i) {
                vector.push(m.success_rate as f32);
                vector.push((m.latency_ms / 10000.0).min(1.0) as f32);
                vector.push((m.token_consumption as f32 / 10000.0).min(1.0));
            } else {
                vector.push(0.0);
                vector.push(0.0);
                vector.push(0.0);
            }
        }

        vector.push((state.history_optimization_count as f32 / 20.0).min(1.0));
        vector.push((state.current_version_age_days as f32 / 30.0).min(1.0));
        vector.push((state.current_timeout_ms as f32 / 60000.0).min(1.0));
        vector.push(state.current_retry_count as f32 / 10.0);

        vector
    }

    async fn call_rl_model(&self, state: &[f32]) -> anyhow::Result<RLModelResponse> {
        let client = reqwest::Client::new();
        
        let response = client
            .post(&self.model_endpoint)
            .json(&serde_json::json!({ "state": state }))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("RL model returned status {}", response.status()));
        }

        Ok(response.json().await?)
    }

    pub async fn get_decision_history(&self) -> Vec<RLDecision> {
        self.decision_history.read().await.clone()
    }

    pub async fn get_stats(&self) -> RLStats {
        let decisions = self.decision_history.read().await;
        
        let mut action_counts = std::collections::HashMap::new();
        for d in decisions.iter() {
            *action_counts.entry(d.action.as_str().to_string()).or_insert(0) += 1;
        }

        RLStats {
            total_decisions: decisions.len(),
            action_counts,
            avg_confidence: if decisions.is_empty() {
                0.0
            } else {
                decisions.iter().map(|d| d.confidence).sum::<f32>() / decisions.len() as f32
            },
        }
    }

    pub async fn apply_decision(&self, decision: &RLDecision) -> DecisionApplyResult {
        match decision.action {
            RLAction::Promote => DecisionApplyResult {
                success: true,
                changes_applied: vec!["version_promoted".to_string()],
            },
            RLAction::Rollback => DecisionApplyResult {
                success: true,
                changes_applied: vec!["version_rollback".to_string()],
            },
            RLAction::AdjustTimeout => DecisionApplyResult {
                success: true,
                changes_applied: vec!["timeout_increased".to_string()],
            },
            RLAction::AdjustRetry => DecisionApplyResult {
                success: true,
                changes_applied: vec!["retry_count_increased".to_string()],
            },
            RLAction::DoNothing => DecisionApplyResult {
                success: true,
                changes_applied: vec![],
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLModelResponse {
    pub action: String,
    pub confidence: f32,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionApplyResult {
    pub success: bool,
    pub changes_applied: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLStats {
    pub total_decisions: usize,
    pub action_counts: std::collections::HashMap<String, u32>,
    pub avg_confidence: f32,
}

pub fn create_rl_decision_maker(endpoint: String) -> RLDecisionMaker {
    RLDecisionMaker::new(endpoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rule_based_decision() {
        let maker = RLDecisionMaker::new("http://localhost:5000/decide".to_string());
        
        let state = RLState {
            execution_history: VecDeque::from(vec![
                ExecutionMetrics { success_rate: 0.6, latency_ms: 2000, token_consumption: 5000, timestamp: 0 },
                ExecutionMetrics { success_rate: 0.7, latency_ms: 1500, token_consumption: 4500, timestamp: 0 },
            ]),
            history_optimization_count: 3,
            current_version_age_days: 10,
            current_timeout_ms: 30000,
            current_retry_count: 3,
        };

        let decision = maker.rule_based_decision(state).await;
        assert_eq!(decision.action, RLAction::Rollback);
    }
}