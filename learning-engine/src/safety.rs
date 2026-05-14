use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SafetyCheckType {
    SchemaValidation,
    SecurityScan,
    ResourceLimit,
    DependencyCheck,
    CodeAnalysis,
    NetworkAccess,
    EnvironmentVariable,
    SecretExposure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRule {
    pub check_type: SafetyCheckType,
    pub pattern: String,
    pub severity: SafetySeverity,
    pub description: String,
    pub auto_whitelistable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SafetySeverity {
    Info,
    Warning,
    Critical,
    Blocker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyCheckResult {
    pub check_type: SafetyCheckType,
    pub passed: bool,
    pub severity: SafetySeverity,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyReport {
    pub workflow_id: String,
    pub version: u32,
    pub checks: Vec<SafetyCheckResult>,
    pub overall_safe: bool,
    pub blocked: bool,
    pub needs_review: bool,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct WhitelistEntry {
    pub entity_type: EntityType,
    pub entity_value: String,
    pub reason: String,
    pub added_at: i64,
    pub added_by: String,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EntityType {
    Workflow(String),
    NodeType(String),
    Pattern(String),
    IpRange(String),
    Environment(String),
    SecretPattern(String),
}

impl WhitelistEntry {
    pub fn new(entity_type: EntityType, entity_value: String, reason: String, added_by: String) -> Self {
        Self {
            entity_type,
            entity_value,
            reason,
            added_at: chrono::Utc::now().timestamp(),
            added_by,
            expires_at: None,
        }
    }

    pub fn with_expiry(mut self, expires_at: i64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.expires_at {
            return chrono::Utc::now().timestamp() > exp;
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistStats {
    pub total_entries: usize,
    pub active_entries: usize,
    pub expired_entries: usize,
    pub entries_by_type: std::collections::HashMap<String, usize>,
}

pub struct SafetyWhitelist {
    entries: Arc<RwLock<HashSet<WhitelistEntry>>>,
    rules: Arc<RwLock<Vec<SafetyRule>>>,
    auto_approve_enabled: bool,
}

impl SafetyWhitelist {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashSet::new())),
            rules: Arc::new(RwLock::new(Vec::new())),
            auto_approve_enabled: true,
        }
    }

    pub fn with_auto_approve(enabled: bool) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashSet::new())),
            rules: Arc::new(RwLock::new(Vec::new())),
            auto_approve_enabled: enabled,
        }
    }

    pub async fn add_entry(&self, entry: WhitelistEntry) {
        let mut entries = self.entries.write().await;
        entries.insert(entry);
    }

    pub async fn remove_entry(&self, entry: &WhitelistEntry) {
        let mut entries = self.entries.write().await;
        entries.remove(entry);
    }

    pub async fn is_whitelisted(&self, entity_type: &EntityType, value: &str) -> bool {
        let entries = self.entries.read().await;
        entries.iter().any(|e| {
            &e.entity_type == entity_type && e.entity_value == value && !e.is_expired()
        })
    }

    pub async fn get_stats(&self) -> WhitelistStats {
        let entries = self.entries.read().await;
        let mut active = 0;
        let mut expired = 0;
        let mut by_type = std::collections::HashMap::new();

        for entry in entries.iter() {
            if entry.is_expired() {
                expired += 1;
            } else {
                active += 1;
            }
            let type_key = format!("{:?}", entry.entity_type);
            *by_type.entry(type_key).or_insert(0) += 1;
        }

        WhitelistStats {
            total_entries: entries.len(),
            active_entries: active,
            expired_entries: expired,
            entries_by_type: by_type,
        }
    }

    pub async fn cleanup_expired(&self) -> usize {
        let mut entries = self.entries.write().await;
        let before = entries.len();
        entries.retain(|e| !e.is_expired());
        before - entries.len()
    }

    pub async fn add_rule(&self, rule: SafetyRule) {
        let mut rules = self.rules.write().await;
        rules.push(rule);
    }

    pub async fn get_rules(&self) -> Vec<SafetyRule> {
        self.rules.read().await.clone()
    }

    pub fn is_auto_approve_enabled(&self) -> bool {
        self.auto_approve_enabled
    }

    pub fn set_auto_approve(&mut self, enabled: bool) {
        self.auto_approve_enabled = enabled;
    }
}

impl Default for SafetyWhitelist {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SafetyChecker {
    whitelist: Arc<SafetyWhitelist>,
    custom_checks: Vec<Box<dyn Send + Sync + Fn(&str) -> Vec<SafetyCheckResult>>>,
}

impl SafetyChecker {
    pub fn new(whitelist: Arc<SafetyWhitelist>) -> Self {
        Self {
            whitelist,
            custom_checks: Vec::new(),
        }
    }

    pub fn add_custom_check(
        &mut self,
        check: impl Send + Sync + Fn(&str) -> Vec<SafetyCheckResult> + 'static,
    ) {
        self.custom_checks.push(Box::new(check));
    }

    pub async fn check_workflow(&self, workflow_yaml: &str) -> SafetyReport {
        let mut checks = Vec::new();
        
        checks.push(self.check_secrets(workflow_yaml).await);
        checks.push(self.check_network_access(workflow_yaml).await);
        checks.push(self.check_resource_limits(workflow_yaml).await);
        checks.push(self.check_env_variables(workflow_yaml).await);

        for custom_check in &self.custom_checks {
            checks.extend(custom_check(workflow_yaml));
        }

        let overall_safe = checks.iter().all(|c| c.passed);
        let blocked = checks.iter().any(|c| c.severity == SafetySeverity::Blocker && !c.passed);
        let needs_review = checks.iter().any(|c| 
            c.severity == SafetySeverity::Warning && !c.passed
        ) && !self.whitelist.is_auto_approve_enabled();

        SafetyReport {
            workflow_id: "workflow".to_string(),
            version: 1,
            checks,
            overall_safe,
            blocked,
            needs_review,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    async fn check_secrets(&self, yaml: &str) -> SafetyCheckResult {
        let secret_patterns = ["password", "token", "secret", "api_key", "private_key"];
        let found: Vec<&str> = secret_patterns.iter()
            .filter(|p| yaml.to_lowercase().contains(*p))
            .copied()
            .collect();

        if !found.is_empty() {
            let is_whitelisted = self.whitelist.is_whitelisted(
                &EntityType::SecretPattern("secret_exposure".to_string()),
                &found.join(","),
            ).await;

            SafetyCheckResult {
                check_type: SafetyCheckType::SecretExposure,
                passed: is_whitelisted || found.is_empty(),
                severity: if is_whitelisted { SafetySeverity::Info } else { SafetySeverity::Critical },
                message: if is_whitelisted {
                    "Secrets pattern found but whitelisted".to_string()
                } else {
                    format!("Found secret patterns: {:?}", found)
                },
                details: Some(serde_json::json!({ "patterns": found })),
            }
        } else {
            SafetyCheckResult {
                check_type: SafetyCheckType::SecretExposure,
                passed: true,
                severity: SafetySeverity::Info,
                message: "No secret patterns found".to_string(),
                details: None,
            }
        }
    }

    async fn check_network_access(&self, yaml: &str) -> SafetyCheckResult {
        let ip_patterns = ["192.168.", "10.", "172.16.", "localhost", "127.0.0.1"];
        let external_patterns = ["0.0.0.0", "::", "http://", "https://"];
        
        let has_internal = ip_patterns.iter().any(|p| yaml.contains(p));
        let has_external = external_patterns.iter().any(|p| yaml.contains(p));

        let is_whitelisted = self.whitelist.is_whitelisted(
            &EntityType::IpRange("network_access".to_string()),
            if has_external { "external" } else { "internal" },
        ).await;

        SafetyCheckResult {
            check_type: SafetyCheckType::NetworkAccess,
            passed: is_whitelisted || !has_external,
            severity: SafetySeverity::Warning,
            message: if has_external {
                "External network access detected".to_string()
            } else {
                "Network access is internal only".to_string()
            },
            details: Some(serde_json::json!({
                "has_internal": has_internal,
                "has_external": has_external,
                "whitelisted": is_whitelisted
            })),
        }
    }

    async fn check_resource_limits(&self, yaml: &str) -> SafetyCheckResult {
        let memory_limit = self.extract_value(yaml, "memory");
        let timeout = self.extract_value(yaml, "timeout");

        let has_limits = memory_limit.is_some() || timeout.is_some();

        SafetyCheckResult {
            check_type: SafetyCheckType::ResourceLimit,
            passed: true,
            severity: if has_limits { SafetySeverity::Info } else { SafetySeverity::Warning },
            message: if has_limits {
                "Resource limits configured".to_string()
            } else {
                "No resource limits found - recommend adding limits".to_string()
            },
            details: Some(serde_json::json!({
                "memory_limit": memory_limit,
                "timeout": timeout
            })),
        }
    }

    async fn check_env_variables(&self, yaml: &str) -> SafetyCheckResult {
        let env_section = yaml.contains("environment:") || yaml.contains("env:");
        
        SafetyCheckResult {
            check_type: SafetyCheckType::EnvironmentVariable,
            passed: true,
            severity: SafetySeverity::Info,
            message: if env_section {
                "Environment variables section found".to_string()
            } else {
                "No environment variables".to_string()
            },
            details: Some(serde_json::json!({ "has_env": env_section })),
        }
    }

    fn extract_value(&self, yaml: &str, key: &str) -> Option<String> {
        let pattern = format!("{}:", key);
        if yaml.contains(&pattern) {
            Some(pattern)
        } else {
            None
        }
    }

    pub async fn can_auto_approve(&self, report: &SafetyReport) -> bool {
        if !self.whitelist.is_auto_approve_enabled() {
            return false;
        }

        if report.blocked {
            return false;
        }

        if report.needs_review {
            return false;
        }

        true
    }
}

pub fn create_whitelist_entry(
    entity_type: EntityType,
    value: String,
    reason: String,
    added_by: String,
) -> WhitelistEntry {
    WhitelistEntry::new(entity_type, value, reason, added_by)
}

pub async fn auto_approve_if_safe(
    checker: &SafetyChecker,
    report: &SafetyReport,
) -> bool {
    checker.can_auto_approve(report).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_whitelist_entry() {
        let whitelist = SafetyWhitelist::new();
        let entry = WhitelistEntry::new(
            EntityType::Workflow("test-wf".to_string()),
            "test-value".to_string(),
            "Testing".to_string(),
            "system".to_string(),
        );
        
        whitelist.add_entry(entry).await;
        let is_listed = whitelist.is_whitelisted(&EntityType::Workflow("test-wf".to_string()), "test-value").await;
        assert!(is_listed);
    }

    #[tokio::test]
    async fn test_safety_check() {
        let whitelist = Arc::new(SafetyWhitelist::new());
        let checker = SafetyChecker::new(whitelist);
        let report = checker.check_workflow("name: test-workflow").await;
        assert!(report.overall_safe);
    }
}