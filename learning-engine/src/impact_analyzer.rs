use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationImpact {
    pub optimization_id: String,
    pub workflow_id: String,
    pub version_from: u32,
    pub version_to: u32,
    pub timestamp: i64,
    pub metrics: ImpactMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactMetrics {
    pub token_change: i64,
    pub latency_change_ms: i64,
    pub success_rate_change: f64,
    pub manual_interventions_saved: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyReport {
    pub week_start: i64,
    pub week_end: i64,
    pub total_optimizations: u32,
    pub total_tokens_saved: i64,
    pub total_cost_saved_usd: f64,
    pub avg_latency_change_ms: f64,
    pub avg_success_rate_change: f64,
    pub best_case: Option<BestCase>,
    pub breakdown: Vec<WorkflowImpact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowImpact {
    pub workflow_id: String,
    pub optimization_count: u32,
    pub tokens_saved: i64,
    pub cost_saved_usd: f64,
    pub success_rate_improvement: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestCase {
    pub workflow_id: String,
    pub description: String,
    pub improvement_percent: f64,
    pub cost_saved_usd: f64,
}

pub struct ImpactAnalyzer {
    impacts: Arc<RwLock<Vec<OptimizationImpact>>>,
    cost_per_token: f64,
    cost_per_manual_hour: f64,
}

impl ImpactAnalyzer {
    pub fn new(cost_per_token: f64, cost_per_manual_hour: f64) -> Self {
        Self {
            impacts: Arc::new(RwLock::new(Vec::new())),
            cost_per_token,
            cost_per_manual_hour,
        }
    }

    pub async fn record_optimization(
        &self,
        workflow_id: &str,
        version_from: u32,
        version_to: u32,
        before_metrics: &WorkflowMetrics,
        after_metrics: &WorkflowMetrics,
    ) -> OptimizationImpact {
        let token_change = before_metrics.total_tokens - after_metrics.total_tokens;
        let latency_change = after_metrics.avg_latency_ms - before_metrics.avg_latency_ms;
        let success_rate_change = after_metrics.success_rate - before_metrics.success_rate;

        let impact = OptimizationImpact {
            optimization_id: format!("opt_{}", chrono::Utc::now().timestamp()),
            workflow_id: workflow_id.to_string(),
            version_from,
            version_to,
            timestamp: chrono::Utc::now().timestamp(),
            metrics: ImpactMetrics {
                token_change,
                latency_change_ms: latency_change,
                success_rate_change,
                manual_interventions_saved: 0,
            },
        };

        self.impacts.write().await.push(impact.clone());
        impact
    }

    pub async fn get_weekly_report(&self, weeks_ago: u32) -> WeeklyReport {
        let now = chrono::Utc::now().timestamp();
        let week_ms = 7 * 24 * 3600 * 1000;
        let week_start = now - (weeks_ago as i64 + 1) * week_ms;
        let week_end = now - weeks_ago as i64 * week_ms;

        let impacts = self.impacts.read().await;
        let week_impacts: Vec<_> = impacts
            .iter()
            .filter(|i| i.timestamp >= week_start && i.timestamp < week_end)
            .collect();

        let total_tokens_saved: i64 = week_impacts.iter().map(|i| -i.metrics.token_change).sum();
        let total_cost = (total_tokens_saved as f64) * self.cost_per_token;
        
        let avg_latency: f64 = if !week_impacts.is_empty() {
            week_impacts.iter().map(|i| i.metrics.latency_change_ms as f64).sum::<f64>() / week_impacts.len() as f64
        } else {
            0.0
        };
        
        let avg_success: f64 = if !week_impacts.is_empty() {
            week_impacts.iter().map(|i| i.metrics.success_rate_change).sum::<f64>() / week_impacts.len() as f64
        } else {
            0.0
        };

        let mut by_workflow: HashMap<String, Vec<_>> = HashMap::new();
        for i in &week_impacts {
            by_workflow.entry(i.workflow_id.clone()).or_default().push(i);
        }

        let breakdown: Vec<WorkflowImpact> = by_workflow
            .iter()
            .map(|(wf_id, opts)| {
                let tokens: i64 = opts.iter().map(|i| -i.metrics.token_change).sum();
                WorkflowImpact {
                    workflow_id: wf_id.clone(),
                    optimization_count: opts.len() as u32,
                    tokens_saved: tokens,
                    cost_saved_usd: (tokens as f64) * self.cost_per_token,
                    success_rate_improvement: opts.iter().map(|i| i.metrics.success_rate_change).sum::<f64>() / opts.len() as f64,
                }
            })
            .collect();

        let best_case = breakdown
            .iter()
            .max_by(|a, b| a.cost_saved_usd.partial_cmp(&b.cost_saved_usd).unwrap())
            .map(|w| BestCase {
                workflow_id: w.workflow_id.clone(),
                description: format!("Optimization saved ${:.2}", w.cost_saved_usd),
                improvement_percent: w.success_rate_improvement * 100.0,
                cost_saved_usd: w.cost_saved_usd,
            });

        WeeklyReport {
            week_start,
            week_end,
            total_optimizations: week_impacts.len() as u32,
            total_tokens_saved,
            total_cost_saved_usd: total_cost,
            avg_latency_change_ms: avg_latency,
            avg_success_rate_change: avg_success,
            best_case,
            breakdown,
        }
    }

    pub async fn get_all_impacts(&self) -> Vec<OptimizationImpact> {
        self.impacts.read().await.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetrics {
    pub total_tokens: i64,
    pub avg_latency_ms: i64,
    pub success_rate: f64,
}

impl Default for WorkflowMetrics {
    fn default() -> Self {
        Self {
            total_tokens: 0,
            avg_latency_ms: 0,
            success_rate: 0.0,
        }
    }
}

pub fn create_impact_analyzer() -> ImpactAnalyzer {
    ImpactAnalyzer::new(0.001, 50.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_impact_calculation() {
        let analyzer = ImpactAnalyzer::new(0.001, 50.0);
        
        let before = WorkflowMetrics {
            total_tokens: 10000,
            avg_latency_ms: 500,
            success_rate: 0.8,
        };
        let after = WorkflowMetrics {
            total_tokens: 8000,
            avg_latency_ms: 400,
            success_rate: 0.9,
        };

        let impact = analyzer.record_optimization("wf1", 1, 2, &before, &after).await;
        
        assert_eq!(impact.metrics.token_change, -2000);
        assert_eq!(impact.metrics.success_rate_change, 0.1);
    }

    #[tokio::test]
    async fn test_weekly_report() {
        let analyzer = ImpactAnalyzer::new(0.001, 50.0);
        let report = analyzer.get_weekly_report(0).await;
        
        assert!(report.total_cost_saved_usd >= 0.0);
    }
}