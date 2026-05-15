use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub id: String,
    pub pattern_id: String,
    pub user_request: String,
    pub accepted: bool,
    pub modifications: Option<serde_json::Value>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternWeight {
    pub pattern_id: String,
    pub weight: f64,
    pub positive_count: i32,
    pub negative_count: i32,
    pub last_updated: i64,
}

pub struct FeedbackCollector {
    pattern_weights: HashMap<String, PatternWeight>,
}

impl FeedbackCollector {
    pub fn new() -> Self {
        Self {
            pattern_weights: HashMap::new(),
        }
    }

    pub fn record_feedback(
        &mut self,
        pattern_id: &str,
        user_request: &str,
        accepted: bool,
        modifications: Option<serde_json::Value>,
    ) -> Feedback {
        let now = chrono::Utc::now().timestamp();
        let feedback = Feedback {
            id: uuid::Uuid::new_v4().to_string(),
            pattern_id: pattern_id.to_string(),
            user_request: user_request.to_string(),
            accepted,
            modifications,
            created_at: now,
        };

        let entry = self
            .pattern_weights
            .entry(pattern_id.to_string())
            .or_insert(PatternWeight {
                pattern_id: pattern_id.to_string(),
                weight: 1.0,
                positive_count: 0,
                negative_count: 0,
                last_updated: now,
            });

        if accepted {
            entry.positive_count += 1;
            entry.weight = (entry.weight + 0.05).min(1.5);
        } else {
            entry.negative_count += 1;
            entry.weight = (entry.weight - 0.03).max(0.5);
        }
        entry.last_updated = now;

        println!(
            "[Feedback] Recorded for pattern '{}': accepted={}, new_weight={:.2}",
            pattern_id, accepted, entry.weight
        );

        feedback
    }

    pub fn get_pattern_weight(&self, pattern_id: &str) -> f64 {
        self.pattern_weights
            .get(pattern_id)
            .map(|p| p.weight)
            .unwrap_or(1.0)
    }

    pub fn get_all_weights(&self) -> Vec<PatternWeight> {
        self.pattern_weights.values().cloned().collect()
    }

    pub fn get_statistics(&self) -> FeedbackStats {
        let mut total_positive = 0;
        let mut total_negative = 0;
        let mut patterns_with_feedback = 0;

        for p in self.pattern_weights.values() {
            total_positive += p.positive_count;
            total_negative += p.negative_count;
            if p.positive_count > 0 || p.negative_count > 0 {
                patterns_with_feedback += 1;
            }
        }

        FeedbackStats {
            total_feedback: total_positive + total_negative,
            positive: total_positive,
            negative: total_negative,
            patterns_with_feedback,
        }
    }
}

impl Default for FeedbackCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackStats {
    pub total_feedback: i32,
    pub positive: i32,
    pub negative: i32,
    pub patterns_with_feedback: i32,
}

pub fn apply_weight_to_score(base_score: f64, pattern_weight: f64) -> f64 {
    base_score * pattern_weight
}

pub fn rank_patterns_with_feedback(
    patterns: Vec<(String, f64)>,
    weights: &HashMap<String, f64>,
) -> Vec<(String, f64)> {
    let mut weighted: Vec<(String, f64)> = patterns
        .into_iter()
        .map(|(id, score)| {
            let weight = weights.get(&id).copied().unwrap_or(1.0);
            (id, score * weight)
        })
        .collect();

    weighted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    weighted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_recording() {
        let mut collector = FeedbackCollector::new();

        collector.record_feedback("P001", "定时抓取 RSS", true, None);
        assert_eq!(collector.get_pattern_weight("P001"), 1.05);

        collector.record_feedback("P001", "定时抓取 RSS", false, None);
        assert_eq!(collector.get_pattern_weight("P001"), 1.02);
    }

    #[test]
    fn test_weight_limits() {
        let mut collector = FeedbackCollector::new();

        for _ in 0..20 {
            collector.record_feedback("P001", "test", true, None);
        }
        assert!(collector.get_pattern_weight("P001") <= 1.5);
    }

    #[test]
    fn test_ranking() {
        let mut weights = HashMap::new();
        weights.insert("P001".to_string(), 1.0);
        weights.insert("P002".to_string(), 1.5);

        let patterns = vec![("P001".to_string(), 0.8), ("P002".to_string(), 0.7)];

        let ranked = rank_patterns_with_feedback(patterns, &weights);

        assert_eq!(ranked[0].0, "P002");
        assert_eq!(ranked[1].0, "P001");
    }
}
