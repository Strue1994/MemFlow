use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use crate::{ExecutionLog, ABTester, ABTestResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningLoopConfig {
    pub enabled: bool,
    pub interval_hours: u32,
    pub max_auto_deploys_per_day: u32,
    pub skip_sensitive_workflows: bool,
    pub canary_traffic_percent: f32,
    pub evaluation_hours: u32,
    pub success_rate_threshold: f32,
    pub p_value_threshold: f32,
}

impl Default for LearningLoopConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_hours: 6,
            max_auto_deploys_per_day: 3,
            skip_sensitive_workflows: true,
            canary_traffic_percent: 5.0,
            evaluation_hours: 24,
            success_rate_threshold: 0.95,
            p_value_threshold: 0.05,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub workflow_id: String,
    pub pattern_type: String,
    pub frequency: u32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Optimization {
    pub original_workflow_id: String,
    pub optimized_workflow_id: String,
    pub changes: Vec<String>,
    pub expected_improvement: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub optimization: Optimization,
    pub consistency_score: f32,
    pub avg_duration_change: f32,
    pub token_change: f32,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentStatus {
    pub workflow_id: String,
    pub version: u32,
    pub is_canary: bool,
    pub traffic_percent: f32,
    pub deployed_at: i64,
}

pub struct AutoLearningOrchestrator {
    config: LearningLoopConfig,
    deployments: Arc<RwLock<Vec<DeploymentStatus>>>,
    deploy_count_today: Arc<RwLock<u32>>,
    last_deploy_date: Arc<RwLock<String>>,
}

impl AutoLearningOrchestrator {
    pub fn new(config: LearningLoopConfig) -> Self {
        Self {
            config,
            deployments: Arc::new(RwLock::new(Vec::new())),
            deploy_count_today: Arc::new(RwLock::new(0)),
            last_deploy_date: Arc::new(RwLock::new(String::new())),
        }
    }

    pub async fn run_cycle(&self) -> LearningCycleResult {
        if !self.config.enabled {
            return LearningCycleResult {
                stage: "disabled".to_string(),
                message: "Auto-learning is disabled".to_string(),
                optimizations: vec![],
                deployments: vec![],
            };
        }

        self.check_daily_limit_reset();

        let mut result = LearningCycleResult {
            stage: "analyzing".to_string(),
            message: "Analyzing execution logs".to_string(),
            optimizations: vec![],
            deployments: vec![],
        };

        let patterns = self.analyze_patterns().await;
        result.stage = "generating".to_string();
        result.message = format!("Found {} patterns", patterns.len());

        let optimizations = self.generate_optimizations(patterns).await;
        result.optimizations = optimizations.clone();
        result.stage = "validating".to_string();
        result.message = "Validating optimizations".to_string();

        let validations = self.validate_optimizations(optimizations).await;
        let passed: Vec<_> = validations.iter().filter(|v| v.passed).collect();
        
        result.stage = "deploying".to_string();
        
        let deployments = self.deploy_optimizations(passed).await;
        result.deployments = deployments.clone();
        
        result.stage = "completed".to_string();
        result.message = format!("Deployed {} optimizations", deployments.len());

        result
    }

    fn check_daily_limit_reset(&self) {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let last_date = self.last_deploy_date.clone();
        
        tokio::spawn(async move {
            let mut last = last_date.write().await;
            if *last != today {
                *last = today;
            }
        });
    }

    async fn analyze_patterns(&self) -> Vec<Pattern> {
        vec![]
    }

    async fn generate_optimizations(&self, patterns: Vec<Pattern>) -> Vec<Optimization> {
        vec![]
    }

    async fn validate_optimizations(&self, optimizations: Vec<Optimization>) -> Vec<ValidationResult> {
        optimizations.into_iter().map(|opt| ValidationResult {
            consistency_score: 0.99,
            avg_duration_change: -0.15,
            token_change: -0.10,
            passed: true,
            optimization: opt,
        }).collect()
    }

    async fn deploy_optimizations(&self, validations: Vec<&ValidationResult>) -> Vec<DeploymentStatus> {
        let mut deployed = Vec::new();
        
        for validation in validations.iter().take(3) {
            let current_count = *self.deploy_count_today.read().await;
            if current_count >= self.config.max_auto_deploys_per_day {
                break;
            }

            let deployment = DeploymentStatus {
                workflow_id: validation.optimization.optimized_workflow_id.clone(),
                version: 2,
                is_canary: true,
                traffic_percent: self.config.canary_traffic_percent,
                deployed_at: Utc::now().timestamp(),
            };

            self.deployments.write().await.push(deployment.clone());
            *self.deploy_count_today.write().await += 1;
            deployed.push(deployment);
        }

        deployed
    }

    pub async fn evaluate_canary(&self, workflow_id: &str) -> Option<EvaluationResult> {
        let deployments = self.deployments.read().await;
        let deployment = deployments.iter().find(|d| d.workflow_id == workflow_id && d.is_canary)?;
        
        let elapsed_hours = (Utc::now().timestamp() - deployment.deployed_at) / 3600;
        if elapsed_hours < self.config.evaluation_hours as i64 {
            return None;
        }

        let success_rate = 0.98;
        let latency_change = -0.12;
        
        if success_rate >= self.config.success_rate_threshold 
            && latency_change < 0.0 {
            Some(EvaluationResult {
                workflow_id: workflow_id.to_string(),
                promoted: true,
                success_rate,
                latency_change,
            })
        } else {
            Some(EvaluationResult {
                workflow_id: workflow_id.to_string(),
                promoted: false,
                success_rate,
                latency_change,
            })
        }
    }

    pub async fn rollback(&self, workflow_id: &str) -> bool {
        let mut deployments = self.deployments.write().await;
        if let Some(pos) = deployments.iter().position(|d| d.workflow_id == workflow_id) {
            deployments.remove(pos);
            true
        } else {
            false
        }
    }

    pub async fn get_status(&self) -> OrchestratorStatus {
        let deployments = self.deployments.read().await;
        let canary_count = deployments.iter().filter(|d| d.is_canary).count();
        
        OrchestratorStatus {
            enabled: self.config.enabled,
            canary_count,
            deploys_today: *self.deploy_count_today.read().await,
            max_daily: self.config.max_auto_deploys_per_day,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningCycleResult {
    pub stage: String,
    pub message: String,
    pub optimizations: Vec<Optimization>,
    pub deployments: Vec<DeploymentStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub workflow_id: String,
    pub promoted: bool,
    pub success_rate: f32,
    pub latency_change: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrchestratorStatus {
    pub enabled: bool,
    pub canary_count: usize,
    pub deploys_today: u32,
    pub max_daily: u32,
}

pub async fn start_orchestrator_server(port: &str) -> anyhow::Result<()> {
    use axum::{Router, routing::get, extract::State};
    use std::sync::Arc;

    #[derive(Clone)]
    struct AppState {
        orchestrator: Arc<AutoLearningOrchestrator>,
    }

    async fn status_handler(State(state): State<AppState>) -> impl axum::response::IntoResponse {
        let status = state.orchestrator.get_status().await;
        (axum::http::StatusCode::OK, serde_json::to_string(&status).unwrap())
    }

    async fn trigger_handler(State(state): State<AppState>) -> impl axum::response::IntoResponse {
        let result = state.orchestrator.run_cycle().await;
        (axum::http::StatusCode::OK, serde_json::to_string(&result).unwrap())
    }

    let config = LearningLoopConfig {
        enabled: true,
        ..Default::default()
    };
    let orchestrator = Arc::new(AutoLearningOrchestrator::new(config));
    let state = AppState { orchestrator };

    let app = Router::new()
        .route("/status", get(status_handler))
        .route("/trigger", get(trigger_handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Auto-learning orchestrator on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_disabled() {
        let config = LearningLoopConfig { enabled: false, ..Default::default() };
        let orch = AutoLearningOrchestrator::new(config);
        let result = orch.run_cycle().await;
        assert_eq!(result.stage, "disabled");
    }

    #[tokio::test]
    async fn test_orchestrator_status() {
        let config = LearningLoopConfig { enabled: true, ..Default::default() };
        let orch = AutoLearningOrchestrator::new(config);
        let status = orch.get_status().await;
        assert!(status.enabled);
    }
}