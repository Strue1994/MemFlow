use chrono::Utc;
use serde::{Deserialize, Serialize};

const EBBINGHAUS_DECAY_CONSTANT: f32 = 0.1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayConfig {
    pub base_decay_rate: f32,
    pub min_importance_threshold: f32,
    pub days_until_permanent_deletion: i64,
    pub importance_boost_on_recall: f32,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            base_decay_rate: 0.1,
            min_importance_threshold: 0.05,
            days_until_permanent_deletion: 90,
            importance_boost_on_recall: 0.1,
        }
    }
}

pub struct EbbinghausDecay {
    config: DecayConfig,
}

impl EbbinghausDecay {
    pub fn new(config: DecayConfig) -> Self {
        Self { config }
    }

    pub fn calculate_importance(&self, initial: f32, days_since_access: i64) -> f32 {
        if days_since_access <= 0 {
            return initial;
        }

        let decay = (-EBBINGHAUS_DECAY_CONSTANT * days_since_access as f32).exp();
        initial * decay
    }

    pub fn should_delete(&self, current_importance: f32, days_since_access: i64) -> bool {
        current_importance < self.config.min_importance_threshold
            && days_since_access > self.config.days_until_permanent_deletion
    }

    pub fn calculate_decay_for_entry(&self, entry: &impl DecayEntry) -> f32 {
        let days = (Utc::now().timestamp() - entry.last_access()) / 86400;
        self.calculate_importance(entry.initial_importance(), days)
    }

    pub fn boost_on_recall(&self, current: f32) -> f32 {
        (current + self.config.importance_boost_on_recall).min(1.0)
    }

    pub fn decay_all_entries(&self, entries: &mut [DecayableMemory]) -> Vec<String> {
        let mut to_remove = Vec::new();

        for entry in entries.iter_mut() {
            let new_importance = self.calculate_decay_for_entry(entry);
            let days = (Utc::now().timestamp() - entry.last_access) / 86400;

            if self.should_delete(new_importance, days) {
                to_remove.push(entry.id.clone());
            } else {
                entry.current_importance = new_importance;
            }
        }

        to_remove
    }
}

pub trait DecayEntry {
    fn id(&self) -> String;
    fn initial_importance(&self) -> f32;
    fn current_importance(&self) -> f32;
    fn last_access(&self) -> i64;
    fn set_importance(&mut self, importance: f32);
    fn set_last_access(&mut self, timestamp: i64);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayableMemory {
    pub id: String,
    pub content: String,
    pub initial_importance: f32,
    pub current_importance: f32,
    pub last_access: i64,
    pub created_at: i64,
    pub vector: Vec<f32>,
}

impl DecayableMemory {
    pub fn new(content: String, importance: f32) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            initial_importance: importance,
            current_importance: importance,
            last_access: now,
            created_at: now,
            vector: vec![],
        }
    }

    pub fn days_since_access(&self) -> i64 {
        (Utc::now().timestamp() - self.last_access) / 86400
    }
}

impl DecayEntry for DecayableMemory {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn initial_importance(&self) -> f32 {
        self.initial_importance
    }
    fn current_importance(&self) -> f32 {
        self.current_importance
    }
    fn last_access(&self) -> i64 {
        self.last_access
    }

    fn set_importance(&mut self, importance: f32) {
        self.current_importance = importance;
    }

    fn set_last_access(&mut self, timestamp: i64) {
        self.last_access = timestamp;
    }
}

pub struct UserPreferenceDecay {
    base_decay: f32,
}

impl UserPreferenceDecay {
    pub fn new() -> Self {
        Self { base_decay: 0.05 }
    }

    pub fn calculate_preference_decay(&self, preference_strength: f32, days: i64) -> f32 {
        let adjusted_decay = self.base_decay * (1.0 - preference_strength * 0.5);
        (-adjusted_decay * days as f32).exp()
    }
}

impl Default for UserPreferenceDecay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decay_calculation() {
        let config = DecayConfig::default();
        let decay = EbbinghausDecay::new(config);

        let importance_0_days = decay.calculate_importance(1.0, 0);
        assert_eq!(importance_0_days, 1.0);

        let importance_10_days = decay.calculate_importance(1.0, 10);
        assert!(importance_10_days < 1.0);
        assert!(importance_10_days > 0.3);
    }

    #[test]
    fn test_recall_boost() {
        let config = DecayConfig::default();
        let decay = EbbinghausDecay::new(config);

        let boosted = decay.boost_on_recall(0.5);
        assert_eq!(boosted, 0.6);

        let boosted_max = decay.boost_on_recall(0.95);
        assert_eq!(boosted_max, 1.0);
    }
}
