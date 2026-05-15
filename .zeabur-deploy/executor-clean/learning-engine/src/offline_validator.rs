use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalExecution {
    pub id: String,
    pub workflow_id: String,
    pub input_params: HashMap<String, String>,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: i64,
    pub executed_at: i64,
}

pub struct OfflineValidator {
    max_duration_days: i64,
    similarity_threshold: f32,
}

impl OfflineValidator {
    pub fn new() -> Self {
        Self {
            max_duration_days: 7,
            similarity_threshold: 0.95,
        }
    }

    pub fn set_max_duration_days(&mut self, days: i64) {
        self.max_duration_days = days;
    }

    pub fn validate(
        &self,
        historical: &[HistoricalExecution],
        optimized_workflow_id: &str,
        run_optimized: impl Fn(&HashMap<String, String>) -> (String, Option<String>, i64),
    ) -> ValidationReport {
        let cutoff = Utc::now().timestamp() - (self.max_duration_days * 86400);
        let relevant: Vec<_> = historical
            .iter()
            .filter(|h| h.workflow_id == optimized_workflow_id && h.executed_at > cutoff)
            .collect();

        if relevant.is_empty() {
            return ValidationReport {
                workflow_id: optimized_workflow_id.to_string(),
                total_tests: 0,
                passed: false,
                consistency_percent: 0.0,
                avg_duration_change: 0.0,
                token_change: 0.0,
                errors: vec!["No historical data found".to_string()],
            };
        }

        let mut consistent_count = 0;
        let mut total_duration_original = 0i64;
        let mut total_duration_optimized = 0i64;
        let mut errors = Vec::new();

        for hist in &relevant {
            total_duration_original += hist.duration_ms;

            let (output, error, duration) = run_optimized(&hist.input_params);
            total_duration_optimized += duration;

            if let Some(orig_err) = &hist.error {
                if error.is_some() {
                    if orig_err == error.as_ref().unwrap() {
                        consistent_count += 1;
                    }
                } else {
                    errors.push(format!("Expected error '{}' but got success", orig_err));
                }
            } else {
                if error.is_none() && self.outputs_similar(&output, &hist.output) {
                    consistent_count += 1;
                } else if error.is_some() {
                    errors.push(format!("Expected success but got error: {:?}", error));
                }
            }
        }

        let total = relevant.len();
        let consistency_percent = (consistent_count as f32 / total as f32) * 100.0;
        let avg_duration_original = total_duration_original as f32 / total as f32;
        let avg_duration_optimized = total_duration_optimized as f32 / total as f32;
        let duration_change = if avg_duration_original > 0.0 {
            (avg_duration_optimized - avg_duration_original) / avg_duration_original
        } else {
            0.0
        };

        let passed =
            consistency_percent >= self.similarity_threshold * 100.0 && errors.len() < total / 10;

        ValidationReport {
            workflow_id: optimized_workflow_id.to_string(),
            total_tests: total,
            passed,
            consistency_percent,
            avg_duration_change: duration_change * 100.0,
            token_change: 0.0,
            errors,
        }
    }

    fn outputs_similar(&self, a: &str, b: &str) -> bool {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();

        if a_chars.len() != b_chars.len() {
            return false;
        }

        let mut matches = 0;
        for (ca, cb) in a_chars.iter().zip(b_chars.iter()) {
            if ca == cb {
                matches += 1;
            }
        }

        matches as f32 / a_chars.len().max(1) as f32 >= self.similarity_threshold
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub workflow_id: String,
    pub total_tests: usize,
    pub passed: bool,
    pub consistency_percent: f32,
    pub avg_duration_change: f32,
    pub token_change: f32,
    pub errors: Vec<String>,
}

impl Default for OfflineValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_similarity() {
        let validator = OfflineValidator::new();
        assert!(validator.outputs_similar("hello world", "hello world"));
        assert!(validator.outputs_similar("hello", "hello"));
    }

    #[test]
    fn test_validation_with_no_data() {
        let validator = OfflineValidator::new();
        let result = validator.validate(&[], "test-wf", |_| ("output".to_string(), None, 100));
        assert_eq!(result.total_tests, 0);
        assert!(!result.passed);
    }
}
