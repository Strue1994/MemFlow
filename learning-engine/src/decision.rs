use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DecisionOutcome {
    Approve,
    Reject,
    Rollback,
    NeedsReview,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryMetrics {
    pub success_rate: f64,
    pub error_rate: f64,
    pub latency_p50_ms: f64,
    pub latency_p99_ms: f64,
    pub request_count: u64,
    pub error_count: u64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
}

impl Default for CanaryMetrics {
    fn default() -> Self {
        Self {
            success_rate: 0.0,
            error_rate: 0.0,
            latency_p50_ms: 0.0,
            latency_p99_ms: 0.0,
            request_count: 0,
            error_count: 0,
            cpu_usage: 0.0,
            memory_usage: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionThresholds {
    pub min_success_rate: f64,
    pub max_error_rate: f64,
    pub max_latency_p99_ms: f64,
    pub min_request_count: u64,
    pub auto_approve_enabled: bool,
    pub rollback_enabled: bool,
}

impl Default for DecisionThresholds {
    fn default() -> Self {
        Self {
            min_success_rate: 0.95,
            max_error_rate: 0.05,
            max_latency_p99_ms: 1000.0,
            min_request_count: 100,
            auto_approve_enabled: true,
            rollback_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentDecision {
    pub workflow_id: String,
    pub version: u32,
    pub canary_version: Option<u32>,
    pub outcome: DecisionOutcome,
    pub confidence: f32,
    pub reasons: Vec<String>,
    pub metrics: CanaryMetrics,
    pub timestamp: i64,
}

impl DeploymentDecision {
    pub fn new(workflow_id: String, version: u32) -> Self {
        Self {
            workflow_id,
            version,
            canary_version: None,
            outcome: DecisionOutcome::Pending,
            confidence: 0.0,
            reasons: Vec::new(),
            metrics: CanaryMetrics::default(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionHistory {
    pub decisions: Vec<DeploymentDecision>,
    pub total_approved: u64,
    pub total_rejected: u64,
    pub total_rollback: u64,
}

impl Default for DecisionHistory {
    fn default() -> Self {
        Self {
            decisions: Vec::new(),
            total_approved: 0,
            total_rejected: 0,
            total_rollback: 0,
        }
    }
}

pub struct AutoDecisionMaker {
    thresholds: DecisionThresholds,
    history: Arc<RwLock<DecisionHistory>>,
    decision_callback: Option<Arc<dyn Send + Sync + Fn(DeploymentDecision) -> anyhow::Result<()>>>,
}

impl AutoDecisionMaker {
    pub fn new(thresholds: DecisionThresholds) -> Self {
        Self {
            thresholds,
            history: Arc::new(RwLock::new(DecisionHistory::default())),
            decision_callback: None,
        }
    }

    pub fn with_callback(
        thresholds: DecisionThresholds,
        callback: impl Send + Sync + Fn(DeploymentDecision) -> anyhow::Result<()> + 'static,
    ) -> Self {
        Self {
            thresholds,
            history: Arc::new(RwLock::new(DecisionHistory::default())),
            decision_callback: Some(Arc::new(callback)),
        }
    }

    pub async fn evaluate(&self, metrics: CanaryMetrics, workflow_id: &str, version: u32) -> DeploymentDecision {
        let mut decision = DeploymentDecision::new(workflow_id.to_string(), version);
        decision.metrics = metrics.clone();

        let mut reasons = Vec::new();
        let mut reject_count = 0;
        let mut approve_count = 0;

        if metrics.request_count < self.thresholds.min_request_count {
            reasons.push(format!(
                "Insufficient traffic: {} < {} requests",
                metrics.request_count, self.thresholds.min_request_count
            ));
            decision.outcome = DecisionOutcome::Pending;
            decision.confidence = 0.3;
            return self.record_decision(decision).await;
        }

        if metrics.error_rate > self.thresholds.max_error_rate {
            reasons.push(format!(
                "High error rate: {:.2}% > {:.2}%",
                metrics.error_rate * 100.0, self.thresholds.max_error_rate * 100.0
            ));
            reject_count += 1;
        }

        if metrics.success_rate < self.thresholds.min_success_rate {
            reasons.push(format!(
                "Low success rate: {:.2}% < {:.2}%",
                metrics.success_rate * 100.0, self.thresholds.min_success_rate * 100.0
            ));
            reject_count += 1;
        }

        if metrics.latency_p99_ms > self.thresholds.max_latency_p99_ms {
            reasons.push(format!(
                "High P99 latency: {:.0}ms > {:.0}ms",
                metrics.latency_p99_ms, self.thresholds.max_latency_p99_ms
            ));
            reject_count += 1;
        }

        if metrics.cpu_usage > 80.0 {
            reasons.push(format!("High CPU usage: {:.1}%", metrics.cpu_usage));
            reject_count += 1;
        }

        if metrics.memory_usage > 85.0 {
            reasons.push(format!("High memory usage: {:.1}%", metrics.memory_usage));
            reject_count += 1;
        }

        if reject_count == 0 {
            reasons.push("All metrics within acceptable thresholds".to_string());
            approve_count += 1;
        }

        if reject_count > 0 && self.thresholds.rollback_enabled && reject_count >= 2 {
            decision.outcome = DecisionOutcome::Rollback;
            decision.confidence = 0.9;
            decision.reasons = reasons;
        } else if reject_count > 0 {
            decision.outcome = DecisionOutcome::NeedsReview;
            decision.confidence = 0.6;
            decision.reasons = reasons;
        } else if self.thresholds.auto_approve_enabled {
            decision.outcome = DecisionOutcome::Approve;
            decision.confidence = 0.85;
            decision.reasons = reasons;
        } else {
            decision.outcome = DecisionOutcome::NeedsReview;
            decision.confidence = 0.7;
            decision.reasons = reasons;
        }

        self.record_decision(decision).await
    }

    async fn record_decision(&self, decision: DeploymentDecision) -> DeploymentDecision {
        let mut history = self.history.write().await;
        
        match decision.outcome {
            DecisionOutcome::Approve => history.total_approved += 1,
            DecisionOutcome::Reject => history.total_rejected += 1,
            DecisionOutcome::Rollback => history.total_rollback += 1,
            _ => {}
        }

        history.decisions.push(decision.clone());

        if let Some(callback) = &self.decision_callback {
            let _ = callback(decision.clone());
        }

        decision
    }

    pub async fn get_history(&self) -> DecisionHistory {
        self.history.read().await.clone()
    }

    pub async fn update_thresholds(&mut self, thresholds: DecisionThresholds) {
        self.thresholds = thresholds;
    }

    pub fn get_thresholds(&self) -> DecisionThresholds {
        self.thresholds.clone()
    }
}

pub async fn auto_approve_workflow(
    decision_maker: &AutoDecisionMaker,
    metrics: CanaryMetrics,
    workflow_id: &str,
    version: u32,
) -> DeploymentDecision {
    decision_maker.evaluate(metrics, workflow_id, version).await
}

pub fn create_decision_maker(config: Option<DecisionThresholds>) -> AutoDecisionMaker {
    AutoDecisionMaker::new(config.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_approve_decision() {
        let maker = AutoDecisionMaker::new(DecisionThresholds::default());
        let metrics = CanaryMetrics {
            success_rate: 0.99,
            error_rate: 0.01,
            latency_p50_ms: 50.0,
            latency_p99_ms: 200.0,
            request_count: 200,
            error_count: 2,
            cpu_usage: 30.0,
            memory_usage: 40.0,
        };
        
        let decision = maker.evaluate(metrics, "test-wf", 1).await;
        assert_eq!(decision.outcome, DecisionOutcome::Approve);
    }

    #[tokio::test]
    async fn test_rollback_decision() {
        let maker = AutoDecisionMaker::new(DecisionThresholds::default());
        let metrics = CanaryMetrics {
            success_rate: 0.70,
            error_rate: 0.30,
            latency_p50_ms: 500.0,
            latency_p99_ms: 2000.0,
            request_count: 200,
            error_count: 60,
            cpu_usage: 90.0,
            memory_usage: 50.0,
        };
        
        let decision = maker.evaluate(metrics, "test-wf", 1).await;
        assert_eq!(decision.outcome, DecisionOutcome::Rollback);
    }

    #[tokio::test]
    async fn test_pending_decision() {
        let maker = AutoDecisionMaker::new(DecisionThresholds::default());
        let metrics = CanaryMetrics {
            success_rate: 0.99,
            error_rate: 0.01,
            latency_p50_ms: 50.0,
            latency_p99_ms: 200.0,
            request_count: 10,
            error_count: 0,
            cpu_usage: 30.0,
            memory_usage: 40.0,
        };
        
        let decision = maker.evaluate(metrics, "test-wf", 1).await;
        assert_eq!(decision.outcome, DecisionOutcome::Pending);
    }
}