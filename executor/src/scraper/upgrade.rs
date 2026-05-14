use serde::{Deserialize, Serialize};

use crate::scraper::fusion::{KnowledgeUnit, StaleDetector};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeSuggestion {
    pub id: String,
    pub source_unit_id: String,
    pub new_content: String,
    pub confidence: f64,
    pub reason: String,
    pub auto_merge: bool,
    pub status: UpgradeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpgradeStatus {
    Pending,
    Approved,
    Rejected,
    Merged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradePipeline {
    suggestions: Vec<UpgradeSuggestion>,
    stale_detector: StaleDetector,
    history: Vec<UpgradeEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeEvent {
    pub id: String,
    pub event_type: UpgradeEventType,
    pub unit_id: String,
    pub timestamp: i64,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpgradeEventType {
    Suggested,
    Approved,
    Rejected,
    Merged,
    RolledBack,
}

impl UpgradePipeline {
    pub fn new(stale_threshold_days: i64) -> Self {
        Self {
            suggestions: Vec::new(),
            stale_detector: StaleDetector::new(stale_threshold_days),
            history: Vec::new(),
        }
    }

    pub fn generate_suggestion(
        &mut self,
        unit_id: String,
        new_content: String,
        confidence: f64,
    ) -> UpgradeSuggestion {
        let suggestion = UpgradeSuggestion {
            id: format!("sugg_{}_{}", unit_id, chrono::Utc::now().timestamp()),
            source_unit_id: unit_id,
            new_content,
            confidence,
            reason: "Content update available".to_string(),
            auto_merge: confidence > 0.9,
            status: UpgradeStatus::Pending,
        };

        self.suggestions.push(suggestion.clone());

        self.history.push(UpgradeEvent {
            id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
            event_type: UpgradeEventType::Suggested,
            unit_id: suggestion.source_unit_id.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            details: format!("Created suggestion {}", suggestion.id),
        });

        suggestion
    }

    pub fn approve(&mut self, suggestion_id: &str) -> Result<(), String> {
        if let Some(sugg) = self.suggestions.iter_mut().find(|s| s.id == suggestion_id) {
            sugg.status = UpgradeStatus::Approved;
            self.history.push(UpgradeEvent {
                id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
                event_type: UpgradeEventType::Approved,
                unit_id: sugg.source_unit_id.clone(),
                timestamp: chrono::Utc::now().timestamp(),
                details: format!("Approved {}", suggestion_id),
            });
            Ok(())
        } else {
            Err(format!("Suggestion {} not found", suggestion_id))
        }
    }

    pub fn reject(&mut self, suggestion_id: &str) -> Result<(), String> {
        if let Some(sugg) = self.suggestions.iter_mut().find(|s| s.id == suggestion_id) {
            let unit_id = sugg.source_unit_id.clone();
            sugg.status = UpgradeStatus::Rejected;
            self.history.push(UpgradeEvent {
                id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
                event_type: UpgradeEventType::Rejected,
                unit_id,
                timestamp: chrono::Utc::now().timestamp(),
                details: format!("Rejected {}", suggestion_id),
            });
            Ok(())
        } else {
            Err(format!("Suggestion {} not found", suggestion_id))
        }
    }

    pub fn merge(&mut self, suggestion_id: &str) -> Result<KnowledgeUnit, String> {
        let (unit_id, new_content) = {
            let sugg = self
                .suggestions
                .iter()
                .find(|s| s.id == suggestion_id)
                .ok_or_else(|| format!("Suggestion {} not found", suggestion_id))?;

            if sugg.status != UpgradeStatus::Approved && !sugg.auto_merge {
                return Err("Suggestion must be approved before merge".to_string());
            }

            (sugg.source_unit_id.clone(), sugg.new_content.clone())
        };

        if let Some(sugg) = self.suggestions.iter_mut().find(|s| s.id == suggestion_id) {
            sugg.status = UpgradeStatus::Merged;
        }

        self.history.push(UpgradeEvent {
            id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
            event_type: UpgradeEventType::Merged,
            unit_id: unit_id.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            details: format!("Merged {}", suggestion_id),
        });

        Ok(KnowledgeUnit {
            id: unit_id,
            content: new_content,
            source: Some("upgrade".to_string()),
            confidence: 0.8,
            freshness: 0,
            is_stale: false,
            tags: vec![],
            version: 1,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        })
    }

    pub fn rollback(&mut self, unit_id: &str, target_event_id: &str) -> Result<(), String> {
        let target = self
            .history
            .iter()
            .find(|e| e.id == target_event_id && e.unit_id == unit_id)
            .ok_or_else(|| format!("Event {} not found", target_event_id))?;

        if target.event_type != UpgradeEventType::Merged {
            return Err("Can only rollback merge events".to_string());
        }

        self.history.push(UpgradeEvent {
            id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
            event_type: UpgradeEventType::RolledBack,
            unit_id: unit_id.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            details: format!("Rolled back to {}", target_event_id),
        });

        Ok(())
    }

    pub fn list_suggestions(&self) -> &[UpgradeSuggestion] {
        &self.suggestions
    }

    pub fn list_history(&self, unit_id: Option<&str>) -> Vec<&UpgradeEvent> {
        match unit_id {
            Some(id) => self.history.iter().filter(|e| &e.unit_id == id).collect(),
            None => self.history.iter().collect(),
        }
    }

    pub fn detect_stale_units(&self, units: &[KnowledgeUnit]) -> Vec<String> {
        self.stale_detector.detect(units)
    }
}
