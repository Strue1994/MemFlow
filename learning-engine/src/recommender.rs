use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWorkflowAction {
    pub user_id: String,
    pub workflow_id: String,
    pub timestamp: i64,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociationRule {
    pub workflow_a: String,
    pub workflow_b: String,
    pub confidence: f64,
    pub support: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub workflow_id: String,
    pub score: f64,
    pub reason: String,
}

pub struct SequenceMiner {
    rules: Arc<RwLock<HashMap<String, Vec<AssociationRule>>>>,
    min_support: f64,
    min_confidence: f64,
}

impl SequenceMiner {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            min_support: 0.1,
            min_confidence: 0.5,
        }
    }

    pub fn with_thresholds(mut self, support: f64, confidence: f64) -> Self {
        self.min_support = support;
        self.min_confidence = confidence;
        self
    }

    pub async fn mine(&self, actions: &[UserWorkflowAction]) -> HashMap<String, Vec<AssociationRule>> {
        let mut sequences: HashMap<String, Vec<(String, i64)>> = HashMap::new();
        
        for action in actions {
            sequences
                .entry(action.user_id.clone())
                .or_default()
                .push((action.workflow_id.clone(), action.timestamp));
        }

        let mut rules: HashMap<String, Vec<AssociationRule>> = HashMap::new();

        for (_, seq) in sequences {
            let mut sorted = seq;
            sorted.sort_by_key(|(_, ts)| *ts);

            let workflow_ids: Vec<String> = sorted.iter().map(|(id, _)| id.clone()).collect();

            for i in 0..workflow_ids.len().saturating_sub(1) {
                let a = &workflow_ids[i];
                let b = &workflow_ids[i + 1];

                let count_ab = self.count_sequence(&workflow_ids[i..], a, b);
                let count_a = self.count_before(&workflow_ids, a, i);
                let support = count_ab as f64 / actions.len() as f64;
                let confidence = if count_a > 0 { count_ab as f64 / count_a as f64 } else { 0.0 };

                if support >= self.min_support && confidence >= self.min_confidence {
                    let rule = AssociationRule {
                        workflow_a: a.clone(),
                        workflow_b: b.clone(),
                        confidence,
                        support,
                    };

                    rules
                        .entry(a.clone())
                        .or_default()
                        .push(rule);
                }
            }
        }

        let mut rules_lock = self.rules.write().await;
        *rules_lock = rules.clone();
        rules
    }

    fn count_sequence(&self, seq: &[String], a: &str, b: &str) -> usize {
        seq.windows(2).filter(|w| w[0] == a && w[1] == b).count()
    }

    fn count_before(&self, seq: &[String], target: &str, before_idx: usize) -> usize {
        seq[..before_idx].iter().filter(|w| *w == target).count()
    }

    pub async fn get_recommendations(&self, workflow_id: &str, limit: usize) -> Vec<AssociationRule> {
        let rules = self.rules.read().await;
        rules.get(workflow_id)
            .map(|r| r.iter().take(limit).cloned().collect())
            .unwrap_or_default()
    }
}

pub struct WorkflowRecommender {
    miner: SequenceMiner,
    user_patterns: Arc<RwLock<HashMap<String, Vec<(i64, String)>>>>,
}

impl WorkflowRecommender {
    pub fn new() -> Self {
        Self {
            miner: SequenceMiner::new(),
            user_patterns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_action(&self, user_id: &str, workflow_id: &str, success: bool, timestamp: i64) {
        let mut patterns = self.user_patterns.write().await;
        patterns
            .entry(user_id.to_string())
            .or_default()
            .push((timestamp, workflow_id.to_string()));

        if patterns.get(user_id).map(|v| v.len()).unwrap_or(0) > 100 {
            patterns.get_mut(user_id).map(|v| v.drain(0..50));
        }
    }

    pub async fn recommend(&self, user_id: &str, last_workflow_id: &str) -> Vec<Recommendation> {
        let rules = self.miner.get_recommendations(last_workflow_id, 5).await;
        
        rules.into_iter()
            .map(|rule| Recommendation {
                workflow_id: rule.workflow_b,
                score: rule.confidence,
                reason: format!("After '{}', {}% users run this", last_workflow_id, rule.confidence * 100.0),
            })
            .collect()
    }

    pub async fn detect_periodic(&self, user_id: &str) -> Option<(String, String)> {
        let patterns = self.user_patterns.read().await;
        let user_patterns = patterns.get(user_id)?;

        if user_patterns.len() < 3 {
            return None;
        }

        let mut hourly_counts = [0usize; 24];
        let mut daily_counts = [0usize; 7];

        for (ts, _) in user_patterns {
            let dt = chrono::DateTime::from_timestamp(ts, 0)?;
            hourly_counts[dt.hour() as usize] += 1;
            daily_counts[dt.weekday().num_days_from_monday() as usize] += 1;
        }

        let peak_hour = hourly_counts.iter().enumerate().max_by_key(|(_, c)| *c)?.0;
        let peak_day = daily_counts.iter().enumerate().max_by_key(|(_, c)| *c)?.0;

        if hourly_counts[peak_hour] >= 3 && daily_counts[peak_day] >= 3 {
            let day_name = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][peak_day];
            let time_str = format!("{} {}:00", day_name, peak_hour);
            return Some((time_str, format!("Detected weekly execution at {}", time_str)));
        }

        None
    }
}

impl Default for SequenceMiner {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for WorkflowRecommender {
    fn default() -> Self {
        Self::new()
    }
}