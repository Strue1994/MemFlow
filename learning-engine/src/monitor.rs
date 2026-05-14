use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSource {
    Prometheus,
    Datadog,
    CloudWatch,
    Custom { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub source: AlertSource,
    pub severity: AlertSeverity,
    pub workflow_id: String,
    pub message: String,
    pub timestamp: i64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub poll_interval_seconds: u64,
    pub alert_history_limit: usize,
    pub auto_rollback_enabled: bool,
    pub alert_thresholds: AlertThresholds,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            poll_interval_seconds: 30,
            alert_history_limit: 100,
            auto_rollback_enabled: true,
            alert_thresholds: AlertThresholds::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub error_rate_threshold: f64,
    pub latency_p99_threshold_ms: f64,
    pub cpu_threshold_percent: f64,
    pub memory_threshold_percent: f64,
    pub success_rate_min_percent: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            error_rate_threshold: 0.05,
            latency_p99_threshold_ms: 1000.0,
            cpu_threshold_percent: 80.0,
            memory_threshold_percent: 85.0,
            success_rate_min_percent: 95.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetrics {
    pub workflow_id: String,
    pub version: u32,
    pub success_rate: f64,
    pub error_rate: f64,
    pub request_count: u64,
    pub latency_p50_ms: f64,
    pub latency_p99_ms: f64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub active_instances: u32,
    pub timestamp: i64,
}

impl Default for WorkflowMetrics {
    fn default() -> Self {
        Self {
            workflow_id: String::new(),
            version: 0,
            success_rate: 0.0,
            error_rate: 0.0,
            request_count: 0,
            latency_p50_ms: 0.0,
            latency_p99_ms: 0.0,
            cpu_usage: 0.0,
            memory_usage: 0.0,
            active_instances: 0,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackAction {
    pub workflow_id: String,
    pub from_version: u32,
    pub to_version: u32,
    pub reason: String,
    pub triggered_by: String,
    pub timestamp: i64,
    pub success: bool,
    pub error_message: Option<String>,
}

pub struct MonitorService {
    config: MonitoringConfig,
    alerts: Arc<RwLock<Vec<Alert>>>,
    metrics_history: Arc<RwLock<Vec<WorkflowMetrics>>>,
    rollbacks: Arc<RwLock<Vec<RollbackAction>>>,
    rollback_callback: Option<Arc<dyn Send + Sync + Fn(RollbackAction) -> anyhow::Result<()>>>,
}

impl MonitorService {
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            alerts: Arc::new(RwLock::new(Vec::new())),
            metrics_history: Arc::new(RwLock::new(Vec::new())),
            rollbacks: Arc::new(RwLock::new(Vec::new())),
            rollback_callback: None,
        }
    }

    pub fn with_rollback_callback(
        config: MonitoringConfig,
        callback: impl Send + Sync + Fn(RollbackAction) -> anyhow::Result<()> + 'static,
    ) -> Self {
        Self {
            config,
            alerts: Arc::new(RwLock::new(Vec::new())),
            metrics_history: Arc::new(RwLock::new(Vec::new())),
            rollbacks: Arc::new(RwLock::new(Vec::new())),
            rollback_callback: Some(Arc::new(callback)),
        }
    }

    pub async fn check_metrics(&self, metrics: &WorkflowMetrics) -> Vec<Alert> {
        let mut triggered_alerts = Vec::new();
        let thresholds = &self.config.alert_thresholds;

        if metrics.error_rate > thresholds.error_rate_threshold {
            triggered_alerts.push(Alert {
                id: format!("alert_{}_{}", metrics.workflow_id, chrono::Utc::now().timestamp_millis()),
                source: AlertSource::Custom { name: "memflow-monitor".to_string() },
                severity: if metrics.error_rate > thresholds.error_rate_threshold * 2.0 {
                    AlertSeverity::Critical
                } else {
                    AlertSeverity::Warning
                },
                workflow_id: metrics.workflow_id.clone(),
                message: format!("Error rate {:.2}% exceeds threshold {:.2}%", 
                    metrics.error_rate * 100.0, thresholds.error_rate_threshold * 100.0),
                timestamp: chrono::Utc::now().timestamp(),
                metadata: serde_json::json!({ "error_rate": metrics.error_rate }),
            });
        }

        if metrics.latency_p99_ms > thresholds.latency_p99_threshold_ms {
            triggered_alerts.push(Alert {
                id: format!("latency_{}_{}", metrics.workflow_id, chrono::Utc::now().timestamp_millis()),
                source: AlertSource::Custom { name: "memflow-monitor".to_string() },
                severity: AlertSeverity::Warning,
                workflow_id: metrics.workflow_id.clone(),
                message: format!("P99 latency {:.0}ms exceeds threshold {:.0}ms",
                    metrics.latency_p99_ms, thresholds.latency_p99_threshold_ms),
                timestamp: chrono::Utc::now().timestamp(),
                metadata: serde_json::json!({ "latency_p99": metrics.latency_p99_ms }),
            });
        }

        if metrics.cpu_usage > thresholds.cpu_threshold_percent {
            triggered_alerts.push(Alert {
                id: format!("cpu_{}_{}", metrics.workflow_id, chrono::Utc::now().timestamp_millis()),
                source: AlertSource::Custom { name: "memflow-monitor".to_string() },
                severity: AlertSeverity::Warning,
                workflow_id: metrics.workflow_id.clone(),
                message: format!("CPU usage {:.1}% exceeds threshold {:.1}%",
                    metrics.cpu_usage, thresholds.cpu_threshold_percent),
                timestamp: chrono::Utc::now().timestamp(),
                metadata: serde_json::json!({ "cpu_usage": metrics.cpu_usage }),
            });
        }

        if metrics.memory_usage > thresholds.memory_threshold_percent {
            triggered_alerts.push(Alert {
                id: format!("memory_{}_{}", metrics.workflow_id, chrono::Utc::now().timestamp_millis()),
                source: AlertSource::Custom { name: "memflow-monitor".to_string() },
                severity: AlertSeverity::Critical,
                workflow_id: metrics.workflow_id.clone(),
                message: format!("Memory usage {:.1}% exceeds threshold {:.1}%",
                    metrics.memory_usage, thresholds.memory_threshold_percent),
                timestamp: chrono::Utc::now().timestamp(),
                metadata: serde_json::json!({ "memory_usage": metrics.memory_usage }),
            });
        }

        if metrics.success_rate < thresholds.success_rate_min_percent {
            triggered_alerts.push(Alert {
                id: format!("success_{}_{}", metrics.workflow_id, chrono::Utc::now().timestamp_millis()),
                source: AlertSource::Custom { name: "memflow-monitor".to_string() },
                severity: AlertSeverity::Critical,
                workflow_id: metrics.workflow_id.clone(),
                message: format!("Success rate {:.2}% below minimum {:.2}%",
                    metrics.success_rate * 100.0, thresholds.success_rate_min_percent),
                timestamp: chrono::Utc::now().timestamp(),
                metadata: serde_json::json!({ "success_rate": metrics.success_rate }),
            });
        }

        if !triggered_alerts.is_empty() {
            self.record_alerts(triggered_alerts.clone()).await;
        }

        triggered_alerts
    }

    async fn record_alerts(&self, alerts: Vec<Alert>) {
        let mut stored = self.alerts.write().await;
        stored.extend(alerts);
        
        if stored.len() > self.config.alert_history_limit {
            let excess = stored.len() - self.config.alert_history_limit;
            stored.drain(0..excess);
        }
    }

    pub async fn should_rollback(&self, metrics: &WorkflowMetrics) -> bool {
        if !self.config.auto_rollback_enabled {
            return false;
        }

        let thresholds = &self.config.alert_thresholds;
        let critical_violations = [
            metrics.error_rate > thresholds.error_rate_threshold * 2.0,
            metrics.success_rate < thresholds.success_rate_min_percent * 0.8,
            metrics.memory_usage > thresholds.memory_threshold_percent,
        ].iter().filter(|&&v| v).count();

        critical_violations >= 2
    }

    pub async fn trigger_rollback(
        &self,
        workflow_id: &str,
        from_version: u32,
        to_version: u32,
        reason: &str,
    ) -> RollbackAction {
        let action = RollbackAction {
            workflow_id: workflow_id.to_string(),
            from_version,
            to_version,
            reason: reason.to_string(),
            triggered_by: "auto-monitor".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            success: false,
            error_message: None,
        };

        let mut rollbacks = self.rollbacks.write().await;
        rollbacks.push(action.clone());

        if let Some(callback) = &self.rollback_callback {
            let _ = callback(action.clone());
        }

        action
    }

    pub async fn record_metrics(&self, metrics: WorkflowMetrics) {
        let mut history = self.metrics_history.write().await;
        history.push(metrics);
        
        if history.len() > 1000 {
            history.drain(0..500);
        }
    }

    pub async fn get_alerts(&self, workflow_id: Option<&str>) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        if let Some(wf_id) = workflow_id {
            alerts.iter().filter(|a| a.workflow_id == wf_id).cloned().collect()
        } else {
            alerts.clone()
        }
    }

    pub async fn get_rollbacks(&self, workflow_id: Option<&str>) -> Vec<RollbackAction> {
        let rollbacks = self.rollbacks.read().await;
        if let Some(wf_id) = workflow_id {
            rollbacks.iter().filter(|r| r.workflow_id == wf_id).cloned().collect()
        } else {
            rollbacks.clone()
        }
    }

    pub async fn get_latest_metrics(&self, workflow_id: &str) -> Option<WorkflowMetrics> {
        let history = self.metrics_history.read().await;
        history.iter().rfind(|m| m.workflow_id == workflow_id).cloned()
    }
}

pub fn create_monitor_service(config: Option<MonitoringConfig>) -> MonitorService {
    MonitorService::new(config.unwrap_or_default())
}

pub async fn auto_monitor_and_rollback(
    monitor: &MonitorService,
    metrics: WorkflowMetrics,
    previous_version: u32,
) -> Option<RollbackAction> {
    let alerts = monitor.check_metrics(&metrics).await;
    
    if alerts.iter().any(|a| a.severity == AlertSeverity::Critical) && monitor.should_rollback(&metrics).await {
        let rollback = monitor.trigger_rollback(
            &metrics.workflow_id,
            metrics.version,
            previous_version,
            "Automatic rollback triggered due to critical alerts",
        ).await;
        return Some(rollback);
    }
    
    monitor.record_metrics(metrics).await;
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_alert_generation() {
        let monitor = MonitorService::new(MonitoringConfig::default());
        let metrics = WorkflowMetrics {
            workflow_id: "test-wf".to_string(),
            version: 2,
            success_rate: 0.80,
            error_rate: 0.20,
            request_count: 100,
            latency_p50_ms: 50.0,
            latency_p99_ms: 500.0,
            cpu_usage: 50.0,
            memory_usage: 50.0,
            active_instances: 2,
            timestamp: 0,
        };
        
        let alerts = monitor.check_metrics(&metrics).await;
        assert!(!alerts.is_empty());
    }

    #[tokio::test]
    async fn test_rollback_decision() {
        let monitor = MonitorService::new(MonitoringConfig::default());
        let metrics = WorkflowMetrics {
            workflow_id: "test-wf".to_string(),
            version: 2,
            success_rate: 0.50,
            error_rate: 0.50,
            request_count: 100,
            latency_p50_ms: 100.0,
            latency_p99_ms: 2000.0,
            cpu_usage: 90.0,
            memory_usage: 90.0,
            active_instances: 2,
            timestamp: 0,
        };
        
        let should = monitor.should_rollback(&metrics).await;
        assert!(should);
    }
}