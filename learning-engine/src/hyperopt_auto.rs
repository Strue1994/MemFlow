use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hyperparams {
    pub learning_interval_hours: f64,
    pub ab_test_duration_hours: f64,
    pub promote_latency_improvement: f64,
    pub emergency_error_rate_threshold: f64,
    pub min_samples_for_optimization: u32,
    pub rollback_on_degradation: bool,
}

impl Default for Hyperparams {
    fn default() -> Self {
        Self {
            learning_interval_hours: 6.0,
            ab_test_duration_hours: 24.0,
            promote_latency_improvement: 0.10,
            emergency_error_rate_threshold: 0.10,
            min_samples_for_optimization: 100,
            rollback_on_degradation: true,
        }
    }
}

impl Hyperparams {
    pub fn validate(&self) -> Result<(), String> {
        if self.learning_interval_hours < 1.0 || self.learning_interval_hours > 24.0 {
            return Err("learning_interval_hours must be between 1 and 24".to_string());
        }
        if self.ab_test_duration_hours < 1.0 || self.ab_test_duration_hours > 168.0 {
            return Err("ab_test_duration_hours must be between 1 and 168".to_string());
        }
        if self.promote_latency_improvement < 0.0 || self.promote_latency_improvement > 1.0 {
            return Err("promote_latency_improvement must be between 0 and 1".to_string());
        }
        if self.emergency_error_rate_threshold < 0.0 || self.emergency_error_rate_threshold > 1.0 {
            return Err("emergency_error_rate_threshold must be between 0 and 1".to_string());
        }
        Ok(())
    }

    pub fn ranges() -> Vec<(String, f64, f64)> {
        vec![
            ("learning_interval_hours".to_string(), 1.0, 24.0),
            ("ab_test_duration_hours".to_string(), 1.0, 168.0),
            ("promote_latency_improvement".to_string(), 0.0, 1.0),
            ("emergency_error_rate_threshold".to_string(), 0.0, 1.0),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub avg_response_time_ms: f64,
    pub success_rate: f64,
    pub token_consumption: f64,
    pub optimization_count: u32,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub params: Hyperparams,
    pub expected_improvement: f64,
    pub actual_improvement: Option<f64>,
    pub applied_at: Option<i64>,
    pub rolled_back: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperoptConfig {
    pub optimization_interval_days: u32,
    pub min_baseline_period_days: u32,
    pub expected_gain_threshold: f64,
    pub python_server_url: String,
}

impl Default for HyperoptConfig {
    fn default() -> Self {
        Self {
            optimization_interval_days: 7,
            min_baseline_period_days: 7,
            expected_gain_threshold: 0.05,
            python_server_url: "http://localhost:5001".to_string(),
        }
    }
}

pub struct HyperoptAuto {
    config: HyperoptConfig,
    current_params: Arc<RwLock<Hyperparams>>,
    previous_params: Arc<RwLock<Option<Hyperparams>>>,
    metrics_history: Arc<RwLock<Vec<SystemMetrics>>>,
    optimization_history: Arc<RwLock<Vec<OptimizationResult>>>,
    baseline: Arc<RwLock<Option<Baseline>>>,
}

#[derive(Debug, Clone)]
struct Baseline {
    avg_response_time_ms: f64,
    success_rate: f64,
    token_consumption: f64,
    calculated_at: i64,
}

impl HyperoptAuto {
    pub fn new(config: HyperoptConfig) -> Self {
        Self {
            config,
            current_params: Arc::new(RwLock::new(Hyperparams::default())),
            previous_params: Arc::new(RwLock::new(None)),
            metrics_history: Arc::new(RwLock::new(Vec::new())),
            optimization_history: Arc::new(RwLock::new(Vec::new())),
            baseline: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn record_metrics(&self, metrics: SystemMetrics) {
        let mut history = self.metrics_history.write().await;
        history.push(metrics);
        
        if history.len() > 10000 {
            history.drain(0..5000);
        }
    }

    async fn calculate_baseline(&self) -> Option<Baseline> {
        let history = self.metrics_history.read().await;
        let period_ms = self.config.min_baseline_period_days as i64 * 86400000;
        let cutoff = chrono::Utc::now().timestamp_millis() - period_ms;
        
        let recent: Vec<_> = history.iter()
            .filter(|m| m.timestamp > cutoff)
            .collect();

        if recent.len() < 10 {
            return None;
        }

        let avg_response: f64 = recent.iter().map(|m| m.avg_response_time_ms).sum::<f64>() / recent.len() as f64;
        let avg_success: f64 = recent.iter().map(|m| m.success_rate).sum::<f64>() / recent.len() as f64;
        let avg_token: f64 = recent.iter().map(|m| m.token_consumption).sum::<f64>() / recent.len() as f64;

        Some(Baseline {
            avg_response_time_ms: avg_response,
            success_rate: avg_success,
            token_consumption: avg_token,
            calculated_at: chrono::Utc::now().timestamp_millis(),
        })
    }

    pub async fn run_optimization(&self) -> anyhow::Result<Option<OptimizationResult>> {
        let baseline = self.calculate_baseline().await;
        
        if baseline.is_none() {
            return Ok(None);
        }

        let baseline = baseline.unwrap();
        let mut current_baseline = self.baseline.write().await;
        *current_baseline = Some(baseline.clone());
        drop(current_baseline);

        let history = self.metrics_history.read().await;
        let optimization_params = self.prepare_optimization_params(&history).await;
        drop(history);

        let suggested = self.call_python_optimizer(&optimization_params).await?;

        let expected_improvement = self.calculate_expected_improvement(&suggested, &baseline).await;

        if expected_improvement < self.config.expected_gain_threshold {
            return Ok(Some(OptimizationResult {
                params: suggested,
                expected_improvement,
                actual_improvement: None,
                applied_at: None,
                rolled_back: false,
            }));
        }

        let current = self.current_params.read().await.clone();
        *self.previous_params.write().await = Some(current);

        *self.current_params.write().await = suggested.clone();

        let result = OptimizationResult {
            params: suggested,
            expected_improvement,
            actual_improvement: None,
            applied_at: Some(chrono::Utc::now().timestamp()),
            rolled_back: false,
        };

        self.optimization_history.write().await.push(result.clone());

        Ok(Some(result))
    }

    async fn prepare_optimization_params(&self, history: &[SystemMetrics]) -> serde_json::Value {
        let recent: Vec<_> = history.iter().take(1000).collect();
        
        let avg_response = if recent.is_empty() {
            0.0
        } else {
            recent.iter().map(|m| m.avg_response_time_ms).sum::<f64>() / recent.len() as f64
        };
        
        let avg_success = if recent.is_empty() {
            0.0
        } else {
            recent.iter().map(|m| m.success_rate).sum::<f64>() / recent.len() as f64
        };

        let avg_token = if recent.is_empty() {
            0.0
        } else {
            recent.iter().map(|m| m.token_consumption).sum::<f64>() / recent.len() as f64
        };

        serde_json::json!({
            "metrics": {
                "avg_response_time_ms": avg_response,
                "success_rate": avg_success,
                "token_consumption": avg_token
            },
            "params": *self.current_params.read().await,
            "ranges": Hyperparams::ranges()
        })
    }

    async fn call_python_optimizer(&self, params: &serde_json::Value) -> anyhow::Result<Hyperparams> {
        let client = reqwest::Client::new();
        
        let url = format!("{}/optimize", self.config.python_server_url);
        
        let response = client
            .post(&url)
            .json(params)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Python optimizer returned {}", response.status()));
        }

        let result: serde_json::Value = response.json().await?;
        
        let mut params = Hyperparams::default();
        
        if let Some(v) = result.get("learning_interval_hours").and_then(|v| v.as_f64()) {
            params.learning_interval_hours = v;
        }
        if let Some(v) = result.get("ab_test_duration_hours").and_then(|v| v.as_f64()) {
            params.ab_test_duration_hours = v;
        }
        if let Some(v) = result.get("promote_latency_improvement").and_then(|v| v.as_f64()) {
            params.promote_latency_improvement = v;
        }
        if let Some(v) = result.get("emergency_error_rate_threshold").and_then(|v| v.as_f64()) {
            params.emergency_error_rate_threshold = v;
        }

        params.validate().map_err(|e| anyhow::anyhow!(e))?;
        
        Ok(params)
    }

    async fn calculate_expected_improvement(&self, params: &Hyperparams, baseline: &Baseline) -> f64 {
        let current = self.current_params.read().await;
        
        let response_change = (baseline.avg_response_time_ms - baseline.avg_response_time_ms * 0.1) / baseline.avg_response_time_ms;
        let success_change = 0.0_f64; // Hyperparams does not carry a success_rate field
        let token_change = (baseline.token_consumption - baseline.token_consumption * 0.05) / baseline.token_consumption;

        let score = 0.5 * response_change + 0.3 * success_change + 0.2 * token_change;
        
        score.max(0.0)
    }

    pub async fn check_and_rollback(&self) -> anyhow::Result<Option<OptimizationResult>> {
        let baseline = self.baseline.read().await;
        let baseline = match &*baseline {
            Some(b) => b,
            None => return Ok(None),
        };
        drop(baseline);

        let history = self.metrics_history.read().await;
        let recent: Vec<_> = history.iter().take(100).collect();
        
        if recent.len() < 10 {
            return Ok(None);
        }

        let current_avg_response: f64 = recent.iter().map(|m| m.avg_response_time_ms).sum::<f64>() / recent.len() as f64;
        let current_avg_success: f64 = recent.iter().map(|m| m.success_rate).sum::<f64>() / recent.len() as f64;

        let response_degradation = (current_avg_response - baseline.avg_response_time_ms) / baseline.avg_response_time_ms;
        let success_degradation = baseline.success_rate - current_avg_success;

        if response_degradation > 0.2 || success_degradation > 0.1 {
            let previous = self.previous_params.read().await;
            if let Some(prev) = previous.as_ref() {
                *self.current_params.write().await = prev.clone();
                
                let mut history = self.optimization_history.write().await;
                if let Some(last) = history.last_mut() {
                    last.rolled_back = true;
                }

                return Ok(Some(OptimizationResult {
                    params: prev.clone(),
                    expected_improvement: 0.0,
                    actual_improvement: Some(-response_degradation),
                    applied_at: None,
                    rolled_back: true,
                }));
            }
        }

        Ok(None)
    }

    pub async fn get_current_params(&self) -> Hyperparams {
        self.current_params.read().await.clone()
    }

    pub async fn get_optimization_history(&self) -> Vec<OptimizationResult> {
        self.optimization_history.read().await.clone()
    }

    pub async fn manual_update(&self, params: Hyperparams) -> anyhow::Result<()> {
        params.validate().map_err(|e| anyhow::anyhow!(e))?;
        *self.current_params.write().await = params;
        Ok(())
    }
}

pub fn create_hyperopt_auto(config: HyperoptConfig) -> HyperoptAuto {
    HyperoptAuto::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hyperparams_validation() {
        let params = Hyperparams::default();
        assert!(params.validate().is_ok());
    }

    #[tokio::test]
    async fn test_hyperopt_creation() {
        let hyperopt = HyperoptAuto::new(HyperoptConfig::default());
        let params = hyperopt.get_current_params().await;
        assert_eq!(params.learning_interval_hours, 6.0);
    }
}