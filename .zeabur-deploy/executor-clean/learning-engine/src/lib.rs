use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
    pub id: String,
    pub workflow_id: String,
    pub version: u32,
    pub params: String,
    pub result: String,
    pub error: Option<String>,
    pub started_at: i64,
    pub finished_at: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPattern {
    pub workflow_id: String,
    pub node_sequence: Vec<String>,
    pub avg_duration_ms: f64,
    pub success_rate: f64,
    pub frequency: u32,
    pub error_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInsight {
    pub workflow_id: String,
    pub pattern_type: String,
    pub description: String,
    pub confidence: f32,
    pub recommendation: String,
}

pub struct PatternMiner {
    min_frequency: u32,
}

impl PatternMiner {
    pub fn new(min_frequency: u32) -> Self {
        Self { min_frequency }
    }

    pub fn analyze_logs(&self, logs: &[ExecutionLog]) -> Vec<ExecutionPattern> {
        let mut workflow_patterns: HashMap<String, Vec<&ExecutionLog>> = HashMap::new();
        
        for log in logs {
            workflow_patterns.entry(log.workflow_id.clone()).or_default().push(log);
        }

        let mut patterns = Vec::new();
        
        for (workflow_id, workflow_logs) in workflow_patterns {
            if workflow_logs.len() < self.min_frequency as usize {
                continue;
            }

            let total_duration: i64 = workflow_logs.iter().map(|l| l.duration_ms).sum();
            let avg_duration = total_duration as f64 / workflow_logs.len() as f64;

            let success_count = workflow_logs.iter().filter(|l| l.error.is_none()).count();
            let success_rate = success_count as f64 / workflow_logs.len() as f64;

            let error_patterns: Vec<String> = workflow_logs
                .iter()
                .filter_map(|l| l.error.clone())
                .fold(HashMap::new(), |mut acc, e| {
                    *acc.entry(e).or_insert(0) += 1;
                    acc
                })
                .into_iter()
                .filter(|(_, count)| *count >= 2)
                .map(|(err, _)| err)
                .collect();

            patterns.push(ExecutionPattern {
                workflow_id,
                node_sequence: vec![],
                avg_duration_ms: avg_duration,
                success_rate,
                frequency: workflow_logs.len() as u32,
                error_patterns,
            });
        }

        patterns.sort_by(|a, b| b.frequency.cmp(&a.frequency));
        patterns
    }

    pub fn generate_insights(&self, patterns: &[ExecutionPattern]) -> Vec<WorkflowInsight> {
        let mut insights = Vec::new();

        for pattern in patterns {
            if pattern.success_rate < 0.8 {
                insights.push(WorkflowInsight {
                    workflow_id: pattern.workflow_id.clone(),
                    pattern_type: "low_success_rate".to_string(),
                    description: format!("Success rate is {:.1}%", pattern.success_rate * 100.0),
                    confidence: pattern.success_rate as f32,
                    recommendation: "Review error patterns and add error handling nodes".to_string(),
                });
            }

            if pattern.avg_duration_ms > 5000.0 {
                insights.push(WorkflowInsight {
                    workflow_id: pattern.workflow_id.clone(),
                    pattern_type: "slow_execution".to_string(),
                    description: format!("Average duration {:.0}ms", pattern.avg_duration_ms),
                    confidence: 0.9,
                    recommendation: "Consider adding parallel execution or caching".to_string(),
                });
            }

            if !pattern.error_patterns.is_empty() {
                insights.push(WorkflowInsight {
                    workflow_id: pattern.workflow_id.clone(),
                    pattern_type: "recurring_errors".to_string(),
                    description: format!("{} recurring errors", pattern.error_patterns.len()),
                    confidence: 0.85,
                    recommendation: "Add retry logic or error recovery nodes".to_string(),
                });
            }
        }

        insights
    }
}

pub struct WorkflowOptimizer;

impl WorkflowOptimizer {
    pub fn suggest_optimizations(insights: &[WorkflowInsight]) -> Vec<String> {
        let mut suggestions = Vec::new();

        for insight in insights {
            match insight.pattern_type.as_str() {
                "low_success_rate" => {
                    suggestions.push(format!(
                        "Workflow {}: Add error handling and retry logic",
                        insight.workflow_id
                    ));
                }
                "slow_execution" => {
                    suggestions.push(format!(
                        "Workflow {}: Optimize by parallelizing independent nodes",
                        insight.workflow_id
                    ));
                }
                "recurring_errors" => {
                    suggestions.push(format!(
                        "Workflow {}: Implement specific error recovery for: {}",
                        insight.workflow_id,
                        insight.description
                    ));
                }
                _ => {}
            }
        }

        suggestions
    }

    pub fn estimate_improvement(insight: &WorkflowInsight) -> f64 {
        match insight.pattern_type.as_str() {
            "low_success_rate" => 0.15,
            "slow_execution" => 0.30,
            "recurring_errors" => 0.20,
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestConfig {
    pub test_id: String,
    pub workflow_id: String,
    pub variant_a: String,
    pub variant_b: String,
    pub traffic_split: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestResult {
    pub test_id: String,
    pub variant_a_samples: u32,
    pub variant_b_samples: u32,
    pub variant_a_success_rate: f64,
    pub variant_b_success_rate: f64,
    pub variant_a_avg_duration_ms: f64,
    pub variant_b_avg_duration_ms: f64,
    pub winner: Option<String>,
    pub confidence: f32,
}

pub struct ABTester {
    min_samples: u32,
}

impl ABTester {
    pub fn new(min_samples: u32) -> Self {
        Self { min_samples }
    }

    pub fn evaluate(&self, results_a: &[ExecutionLog], results_b: &[ExecutionLog]) -> ABTestResult {
        let variant_a_samples = results_a.len() as u32;
        let variant_b_samples = results_b.len() as u32;

        let variant_a_success = results_a.iter().filter(|l| l.error.is_none()).count() as f64;
        let variant_b_success = results_b.iter().filter(|l| l.error.is_none()).count() as f64;

        let variant_a_success_rate = if variant_a_samples > 0 {
            variant_a_success / variant_a_samples as f64
        } else {
            0.0
        };

        let variant_b_success_rate = if variant_b_samples > 0 {
            variant_b_success / variant_b_samples as f64
        } else {
            0.0
        };

        let variant_a_duration: i64 = results_a.iter().map(|l| l.duration_ms).sum();
        let variant_b_duration: i64 = results_b.iter().map(|l| l.duration_ms).sum();

        let variant_a_avg_duration_ms = if variant_a_samples > 0 {
            variant_a_duration as f64 / variant_a_samples as f64
        } else {
            0.0
        };

        let variant_b_avg_duration_ms = if variant_b_samples > 0 {
            variant_b_duration as f64 / variant_b_samples as f64
        } else {
            0.0
        };

        let winner = if variant_a_samples >= self.min_samples && variant_b_samples >= self.min_samples {
            if variant_a_success_rate > variant_b_success_rate {
                Some("A".to_string())
            } else if variant_b_success_rate > variant_a_success_rate {
                Some("B".to_string())
            } else if variant_a_avg_duration_ms < variant_b_avg_duration_ms {
                Some("A".to_string())
            } else if variant_b_avg_duration_ms < variant_a_avg_duration_ms {
                Some("B".to_string())
            } else {
                None
            }
        } else {
            None
        };

        let confidence = if variant_a_samples > 0 && variant_b_samples > 0 {
            let total = variant_a_samples + variant_b_samples;
            (variant_a_samples.min(variant_b_samples) as f32 / total as f32).min(0.95)
        } else {
            0.0
        };

        ABTestResult {
            test_id: String::new(),
            variant_a_samples,
            variant_b_samples,
            variant_a_success_rate,
            variant_b_success_rate,
            variant_a_avg_duration_ms,
            variant_b_avg_duration_ms,
            winner,
            confidence,
        }
    }
}

pub fn analyze_workflow_performance(logs: &[ExecutionLog]) -> serde_json::Value {
    let total = logs.len();
    if total == 0 {
        return serde_json::json!({
            "total_executions": 0,
            "success_rate": 0.0,
            "avg_duration_ms": 0.0,
            "errors": []
        });
    }

    let success = logs.iter().filter(|l| l.error.is_none()).count();
    let total_duration: i64 = logs.iter().map(|l| l.duration_ms).sum();
    
    let errors: Vec<String> = logs
        .iter()
        .filter_map(|l| l.error.clone())
        .fold(HashMap::new(), |mut acc, e| {
            *acc.entry(e).or_insert(0) += 1;
            acc
        })
        .into_iter()
        .map(|(err, count)| format!("{} ({} times)", err, count))
        .collect();

    serde_json::json!({
        "total_executions": total,
        "success_rate": success as f64 / total as f64,
        "avg_duration_ms": total_duration as f64 / total as f64,
        "errors": errors
    })
}

pub async fn start_analysis_server(port: &str) -> anyhow::Result<()> {
    use axum::{Router, routing::{get, post}, extract::{Query, State}, response::IntoResponse, http::StatusCode};
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use std::time::Duration;
    use tokio::time::interval;

    #[derive(Clone)]
    struct AppState {
        logs: Arc<RwLock<Vec<ExecutionLog>>>,
    }

    #[derive(Deserialize)]
    struct AnalyzeQuery {
        workflow_id: Option<String>,
        min_freq: Option<u32>,
    }

    async fn analyze_handler(
        State(state): State<AppState>,
        Query(query): Query<AnalyzeQuery>,
    ) -> impl IntoResponse {
        let logs = state.logs.read().await;
        let filtered: Vec<_> = if let Some(wf_id) = &query.workflow_id {
            logs.iter().filter(|l| &l.workflow_id == wf_id).cloned().collect()
        } else {
            logs.clone()
        };

        let miner = PatternMiner::new(query.min_freq.unwrap_or(2));
        let patterns = miner.analyze_logs(&filtered);
        let insights = miner.generate_insights(&patterns);
        let suggestions = WorkflowOptimizer::suggest_optimizations(&insights);

        (StatusCode::OK, serde_json::json!({
            "patterns": patterns,
            "insights": insights,
            "suggestions": suggestions
        }).to_string())
    }

    async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
        let count = state.logs.read().await.len();
        (StatusCode::OK, serde_json::json!({ "logs_count": count }).to_string())
    }

    async fn ingest_handler(
        State(state): State<AppState>,
        axum::Json(logs): axum::Json<Vec<ExecutionLog>>,
    ) -> impl IntoResponse {
        let mut state_logs = state.logs.write().await;
        state_logs.extend(logs);
        (StatusCode::OK, "ingested")
    }

    async fn auto_learn_handler(State(state): State<AppState>) -> impl IntoResponse {
        let logs = state.logs.read().await.clone();
        
        let miner = PatternMiner::new(3);
        let patterns = miner.analyze_logs(&logs);
        let insights = miner.generate_insights(&patterns);
        
        let mut learnings = Vec::new();
        for insight in &insights {
            learnings.push(serde_json::json!({
                "workflow_id": insight.workflow_id,
                "pattern_type": insight.pattern_type,
                "recommendation": insight.recommendation,
            }));
        }
        
        (StatusCode::OK, serde_json::json!({ "learnings": learnings }).to_string())
    }

    let state = AppState {
        logs: Arc::new(RwLock::new(Vec::new())),
    };

    let app = Router::new()
        .route("/analyze", get(analyze_handler))
        .route("/health", get(health_handler))
        .route("/ingest", post(ingest_handler))
        .route("/auto-learn", get(auto_learn_handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Learning engine analysis server on {}", addr);
    println!("Auto-learn endpoint: GET /auto-learn");
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_logs(count: usize, success: bool) -> Vec<ExecutionLog> {
        (0..count).map(|i| ExecutionLog {
            id: format!("log_{}", i),
            workflow_id: "test_wf".to_string(),
            version: 1,
            params: "{}".to_string(),
            result: "{}".to_string(),
            error: if success { None } else { Some("Test error".to_string()) },
            started_at: 0,
            finished_at: 100,
            duration_ms: if success { 50 } else { 200 },
        }).collect()
    }

    #[test]
    fn test_pattern_miner() {
        let miner = PatternMiner::new(2);
        let logs = create_test_logs(5, true);
        let patterns = miner.analyze_logs(&logs);
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_ab_tester() {
        let tester = ABTester::new(5);
        let logs_a = create_test_logs(10, true);
        let logs_b = create_test_logs(10, false);
        let result = tester.evaluate(&logs_a, &logs_b);
        assert_eq!(result.winner, Some("A".to_string()));
    }
}

pub mod orchestrator;
pub mod offline_validator;
pub mod auto_deployer;
pub mod param_optimizer;
pub mod strategy_updater;
pub mod log_persistence;
pub mod diff_analyzer;
pub mod prompt_optimizer;
pub mod n8n_validator;
pub mod memory_extractor;
pub mod feedback;

pub use orchestrator::{
    AutoLearningOrchestrator, LearningLoopConfig, LearningCycleResult, 
    EvaluationResult, OrchestratorStatus,
};
pub use offline_validator::{OfflineValidator, HistoricalExecution, ValidationReport};
pub use auto_deployer::{AutoDeployer, DeployRequest, DeployResponse, DeploymentStats};
pub use param_optimizer::{
    OptimizableParam, OptimizationConfig, TargetMetric, Trial, BayesianOptimizer,
    OptimizationResult, ParamOptimizer, calculate_metric, scale_to_param_range,
};
pub use strategy_updater::{
    StrategyUpdater, Context, Action, Decision, PolicyStats, StrategyModel,
    run_online_learning,
};
pub use log_persistence::{ExecutionLogger, LogEntry, ABTestManager, ABTestConfig as ABTestCfg, ABTestEvalResult};
pub use diff_analyzer::{DiffAnalyzer, DiffPatch, WorkflowDiff, ModificationPattern};
pub use prompt_optimizer::{PromptOptimizer, PromptVersion, PromptOptimization, ABTestResult as PromptABTestResult};
pub use n8n_validator::{N8nValidator, N8nWorkflow, N8nValidationError, N8nWorkflowValidation};
pub use feedback::{FeedbackCollector, Feedback, PatternWeight, FeedbackStats, apply_weight_to_score, rank_patterns_with_feedback};