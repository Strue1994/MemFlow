use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub preferred_language: String,
    pub work_style: WorkStyle,
    pub common_patterns: Vec<String>,
    pub skill_preferences: Vec<String>,
    pub interaction_count: u64,
    pub first_seen: i64,
    pub last_active: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkStyle {
    Guided,
    Autonomous,
    Cautious,
    Adaptive,
}

impl Default for WorkStyle { fn default() -> Self { Self::Adaptive } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSignal {
    pub timestamp: i64,
    pub signal_type: String,
    pub value: String,
    pub confidence: f32,
}

pub struct UserModel {
    profiles: HashMap<String, UserProfile>,
    signals: HashMap<String, Vec<UserSignal>>,
}

impl UserModel {
    pub fn new() -> Self {
        Self { profiles: HashMap::new(), signals: HashMap::new() }
    }

    pub fn get_or_create_profile(&mut self, user_id: &str) -> UserProfile {
        self.profiles.get(user_id).cloned().unwrap_or_else(|| {
            let profile = UserProfile {
                user_id: user_id.to_string(),
                preferred_language: "en".to_string(),
                work_style: WorkStyle::Adaptive,
                common_patterns: Vec::new(),
                skill_preferences: Vec::new(),
                interaction_count: 0,
                first_seen: Utc::now().timestamp(),
                last_active: Utc::now().timestamp(),
            };
            self.profiles.insert(user_id.to_string(), profile.clone());
            profile
        })
    }

    pub fn record_signal(&mut self, user_id: &str, signal: UserSignal) {
        self.signals.entry(user_id.to_string()).or_default().push(signal);
        match self.profiles.get_mut(user_id) {
            Some(profile) => {
                profile.interaction_count += 1;
                profile.last_active = Utc::now().timestamp();
            }
            None => {
                let mut p = self.get_or_create_profile(user_id);
                p.interaction_count += 1;
                p.last_active = Utc::now().timestamp();
                self.profiles.insert(user_id.to_string(), p);
            }
        }
    }

    pub fn infer_work_style(&self, user_id: &str) -> WorkStyle {
        let signals = match self.signals.get(user_id) {
            Some(s) => s,
            None => return WorkStyle::Adaptive,
        };
        let auto_c = signals.iter().filter(|s| s.value == "autonomous").count();
        let guided_c = signals.iter().filter(|s| s.value == "guided").count();
        let cautious_c = signals.iter().filter(|s| s.value == "cautious").count();
        if auto_c > guided_c && auto_c > cautious_c { WorkStyle::Autonomous }
        else if guided_c > cautious_c { WorkStyle::Guided }
        else if cautious_c > 0 { WorkStyle::Cautious }
        else { WorkStyle::Adaptive }
    }

    pub fn profile_count(&self) -> usize { self.profiles.len() }
}

impl Default for UserModel { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_creation() {
        let mut model = UserModel::new();
        let profile = model.get_or_create_profile("user_1");
        assert_eq!(profile.user_id, "user_1");
        assert_eq!(profile.interaction_count, 0);
    }

    #[test]
    fn test_signal_recording() {
        let mut model = UserModel::new();
        model.record_signal("user_1", UserSignal {
            timestamp: Utc::now().timestamp(),
            signal_type: "preference".to_string(),
            value: "autonomous".to_string(),
            confidence: 0.8,
        });
        assert_eq!(model.get_or_create_profile("user_1").interaction_count, 1);
    }

    #[test]
    fn test_work_style_inference() {
        let mut model = UserModel::new();
        for _ in 0..5 {
            model.record_signal("user_a", UserSignal {
                timestamp: Utc::now().timestamp(), signal_type: "p".to_string(),
                value: "autonomous".to_string(), confidence: 0.7,
            });
        }
        for _ in 0..2 {
            model.record_signal("user_a", UserSignal {
                timestamp: Utc::now().timestamp(), signal_type: "p".to_string(),
                value: "guided".to_string(), confidence: 0.6,
            });
        }
        assert_eq!(model.infer_work_style("user_a"), WorkStyle::Autonomous);
    }
}



