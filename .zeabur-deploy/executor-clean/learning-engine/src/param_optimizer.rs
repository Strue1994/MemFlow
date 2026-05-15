use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizableParam {
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub step: Option<f64>,
    pub current_value: Option<f64>,
}

impl OptimizableParam {
    pub fn new(name: &str, min: f64, max: f64) -> Self {
        Self {
            name: name.to_string(),
            min,
            max,
            step: None,
            current_value: None,
        }
    }

    pub fn with_step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    pub max_iterations: usize,
    pub exploration_weight: f64,
    pub target_metric: TargetMetric,
    pub num_trials: usize,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            max_iterations: 30,
            exploration_weight: 1.0,
            target_metric: TargetMetric::CompositeDurationError,
            num_trials: 10,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TargetMetric {
    Duration,
    ErrorRate,
    CompositeDurationError,
    TokenCount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trial {
    pub params: HashMap<String, f64>,
    pub metric_value: f64,
    pub success: bool,
    pub duration_ms: i64,
    pub error_count: i32,
}

pub struct BayesianOptimizer {
    config: OptimizationConfig,
    observations: Vec<Trial>,
    param_names: Vec<String>,
}

impl BayesianOptimizer {
    pub fn new(config: OptimizationConfig, params: &[OptimizableParam]) -> Self {
        let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();

        Self {
            config,
            observations: Vec::new(),
            param_names,
        }
    }

    pub fn suggest(&self) -> HashMap<String, f64> {
        let mut rng = rand::thread_rng();
        let mut params = HashMap::new();

        if self.observations.is_empty() {
            for name in &self.param_names {
                params.insert(name.clone(), rng.gen_range(0.0..1.0));
            }
        } else {
            let best = self
                .observations
                .iter()
                .min_by(|a, b| a.metric_value.partial_cmp(&b.metric_value).unwrap())
                .unwrap();

            for (i, name) in self.param_names.iter().enumerate() {
                let noise = rng.gen::<f64>() * self.config.exploration_weight * 0.3;
                let mut value = best.params[name] + noise;
                value = value.clamp(0.0, 1.0);
                params.insert(name.clone(), value);
            }
        }

        params
    }

    pub fn add_observation(&mut self, trial: Trial) {
        self.observations.push(trial);
    }

    pub fn optimize(
        &mut self,
        mut run_trial: impl FnMut(&HashMap<String, f64>) -> Trial,
    ) -> OptimizationResult {
        for _ in 0..self.config.max_iterations {
            let params = self.suggest();
            let trial = run_trial(&params);
            self.add_observation(trial);
        }

        self.get_best_result()
    }

    fn get_best_result(&self) -> OptimizationResult {
        if self.observations.is_empty() {
            return OptimizationResult {
                success: false,
                best_params: HashMap::new(),
                best_metric_value: f64::INFINITY,
                iterations: 0,
                message: "No observations".to_string(),
            };
        }

        let best = self
            .observations
            .iter()
            .min_by(|a, b| a.metric_value.partial_cmp(&b.metric_value).unwrap())
            .unwrap();

        OptimizationResult {
            success: true,
            best_params: best.params.clone(),
            best_metric_value: best.metric_value,
            iterations: self.observations.len(),
            message: format!("Found best in {} trials", self.observations.len()),
        }
    }

    pub fn get_recommendation(&self) -> Option<HashMap<String, f64>> {
        if self.observations.is_empty() {
            return None;
        }

        let best = self
            .observations
            .iter()
            .min_by(|a, b| a.metric_value.partial_cmp(&b.metric_value).unwrap())
            .unwrap();

        Some(best.params.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub success: bool,
    pub best_params: HashMap<String, f64>,
    pub best_metric_value: f64,
    pub iterations: usize,
    pub message: String,
}

pub fn calculate_metric(
    metric_type: TargetMetric,
    duration_ms: i64,
    error_count: i32,
    token_count: Option<i64>,
) -> f32 {
    match metric_type {
        TargetMetric::Duration => duration_ms as f32 / 1000.0,
        TargetMetric::ErrorRate => error_count as f32,
        TargetMetric::CompositeDurationError => {
            0.6 * (duration_ms as f32 / 1000.0) + 0.4 * (error_count as f32 * 10.0)
        }
        TargetMetric::TokenCount => token_count.unwrap_or(0) as f32 / 1000.0,
    }
}

pub fn scale_to_param_range(
    params: &HashMap<String, f64>,
    param_defs: &[OptimizableParam],
) -> HashMap<String, f64> {
    let mut scaled = HashMap::new();

    for def in param_defs {
        if let Some(normalized) = params.get(&def.name) {
            let value = def.min + (def.max - def.min) * normalized;
            let value = if let Some(step) = def.step {
                (value / step).round() * step
            } else {
                value
            };
            scaled.insert(def.name.clone(), value);
        }
    }

    scaled
}

pub struct ParamOptimizer;

impl ParamOptimizer {
    pub fn optimize_workflow_params(
        workflow_id: &str,
        params: Vec<OptimizableParam>,
        run_workflow: impl Fn(&HashMap<String, f64>) -> (i64, i32, Option<i64>),
    ) -> OptimizationResult {
        let config = OptimizationConfig::default();
        let mut optimizer = BayesianOptimizer::new(config, &params);

        optimizer.optimize(|trial_params| {
            let scaled = scale_to_param_range(trial_params, &params);
            let (duration_ms, error_count, token_count) = run_workflow(&scaled);
            let metric = calculate_metric(
                TargetMetric::CompositeDurationError,
                duration_ms,
                error_count,
                token_count,
            );

            Trial {
                params: trial_params.clone(),
                metric_value: metric as f64,
                success: error_count == 0,
                duration_ms,
                error_count,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_param_creation() {
        let param = OptimizableParam::new("timeout", 100.0, 5000.0);
        assert_eq!(param.name, "timeout");
        assert_eq!(param.min, 100.0);
        assert_eq!(param.max, 5000.0);
    }

    #[test]
    fn test_metric_calculation() {
        let metric = calculate_metric(TargetMetric::CompositeDurationError, 1000, 1, None);
        assert!(metric > 0.0);
    }

    #[test]
    fn test_param_scaling() {
        let params = vec![OptimizableParam::new("x", 0.0, 100.0)];
        let mut trial_params = HashMap::new();
        trial_params.insert("x".to_string(), 0.5);

        let scaled = scale_to_param_range(&trial_params, &params);
        assert_eq!(scaled.get("x"), Some(&50.0));
    }
}
