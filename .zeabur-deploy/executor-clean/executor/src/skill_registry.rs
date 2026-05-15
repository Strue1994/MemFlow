use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: SkillCategory,
    pub keywords: Vec<String>,
    pub pattern: Option<String>,
    pub examples: Vec<SkillExample>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillCategory {
    DataProcessing,
    APIIntegration,
    Automation,
    Notification,
    Scheduling,
    AI,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExample {
    pub name: String,
    pub description: String,
    pub workflow_template: Option<serde_json::Value>,
}

pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            skills: HashMap::new(),
        };
        registry.register_builtin_skills();
        registry
    }

    fn register_builtin_skills(&mut self) {
        self.register(Skill {
            id: "http-get".to_string(),
            name: "HTTP GET".to_string(),
            description: "Fetch data from external API".to_string(),
            category: SkillCategory::APIIntegration,
            keywords: vec![
                "http".to_string(),
                "get".to_string(),
                "fetch".to_string(),
                "api".to_string(),
            ],
            pattern: Some("HTTP Request → Transform → Output".to_string()),
            examples: vec![],
            version: "1.0".to_string(),
        });

        self.register(Skill {
            id: "scheduled-task".to_string(),
            name: "Scheduled Task".to_string(),
            description: "Run workflow on a schedule".to_string(),
            category: SkillCategory::Scheduling,
            keywords: vec![
                "schedule".to_string(),
                "cron".to_string(),
                "timer".to_string(),
                "periodic".to_string(),
            ],
            pattern: Some("Schedule Trigger → Actions → Notification".to_string()),
            examples: vec![],
            version: "1.0".to_string(),
        });

        self.register(Skill {
            id: "webhook-handler".to_string(),
            name: "Webhook Handler".to_string(),
            description: "Receive and process webhooks".to_string(),
            category: SkillCategory::Automation,
            keywords: vec![
                "webhook".to_string(),
                "receive".to_string(),
                "callback".to_string(),
                "trigger".to_string(),
            ],
            pattern: Some("Webhook Trigger → Validate → Process → Respond".to_string()),
            examples: vec![],
            version: "1.0".to_string(),
        });

        self.register(Skill {
            id: "ai-agent".to_string(),
            name: "AI Agent".to_string(),
            description: "Build AI agent with tools".to_string(),
            category: SkillCategory::AI,
            keywords: vec![
                "ai".to_string(),
                "agent".to_string(),
                "llm".to_string(),
                "openai".to_string(),
                "claude".to_string(),
            ],
            pattern: Some("Trigger → AI Agent → Tools → Output".to_string()),
            examples: vec![],
            version: "1.0".to_string(),
        });

        self.register(Skill {
            id: "slack-notify".to_string(),
            name: "Slack Notification".to_string(),
            description: "Send notifications to Slack".to_string(),
            category: SkillCategory::Notification,
            keywords: vec![
                "slack".to_string(),
                "notify".to_string(),
                "message".to_string(),
                "alert".to_string(),
            ],
            pattern: Some("Trigger → Slack → Done".to_string()),
            examples: vec![],
            version: "1.0".to_string(),
        });
    }

    fn register(&mut self, skill: Skill) {
        self.skills.insert(skill.id.clone(), skill);
    }

    pub fn get(&self, id: &str) -> Option<&Skill> {
        self.skills.get(id)
    }

    pub fn search(&self, query: &str) -> Vec<&Skill> {
        let query_lower = query.to_lowercase();
        self.skills
            .values()
            .filter(|s| {
                s.name.to_lowercase().contains(&query_lower)
                    || s.description.to_lowercase().contains(&query_lower)
                    || s.keywords
                        .iter()
                        .any(|k| k.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    pub fn list_all(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    pub fn list_by_category(&self, category: &SkillCategory) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| &s.category == category)
            .collect()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn find_matching_skills(user_request: &str) -> Vec<String> {
    let registry = SkillRegistry::new();
    let matches = registry.search(user_request);
    matches.iter().map(|s| s.id.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_registry() {
        let registry = SkillRegistry::new();
        assert!(registry.get("http-get").is_some());
        assert!(registry.search("api").len() > 0);
    }

    #[test]
    fn test_find_matching() {
        let matches = find_matching_skills("I want to fetch data from an API");
        assert!(!matches.is_empty());
    }
}
