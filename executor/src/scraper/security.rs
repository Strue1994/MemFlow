use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SensitivityLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentClassifier {
    sensitive_patterns: Vec<SensitivePattern>,
    allowlist: HashSet<String>,
    blocklist: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivePattern {
    pub pattern: String,
    pub replacement: String,
    pub level: SensitivityLevel,
    pub category: String,
}

impl ContentClassifier {
    pub fn new() -> Self {
        let sensitive_patterns = vec![
            SensitivePattern {
                pattern: r"\b\d{3}-\d{2}-\d{4}\b".to_string(),
                replacement: "[SSN]".to_string(),
                level: SensitivityLevel::Critical,
                category: "ssn".to_string(),
            },
            SensitivePattern {
                pattern: r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b".to_string(),
                replacement: "[EMAIL]".to_string(),
                level: SensitivityLevel::Medium,
                category: "email".to_string(),
            },
            SensitivePattern {
                pattern: r"\b\d{16}\b".to_string(),
                replacement: "[CREDIT_CARD]".to_string(),
                level: SensitivityLevel::Critical,
                category: "credit_card".to_string(),
            },
            SensitivePattern {
                pattern: r"\b\d{10}\b".to_string(),
                replacement: "[PHONE]".to_string(),
                level: SensitivityLevel::Low,
                category: "phone".to_string(),
            },
        ];

        let allowlist = HashSet::new();
        let blocklist = HashSet::from([
            "malware".to_string(),
            "phishing".to_string(),
            "exploit".to_string(),
        ]);

        Self {
            sensitive_patterns,
            allowlist,
            blocklist,
        }
    }

    pub fn classify(&self, content: &str) -> SensitivityLevel {
        for pattern in &self.sensitive_patterns {
            if content.contains(&pattern.pattern) {
                return pattern.level.clone();
            }
        }
        SensitivityLevel::Low
    }

    pub fn redact(&self, content: &str) -> String {
        let mut result = content.to_string();

        for pattern in &self.sensitive_patterns {
            if let Ok(regex) = regex::Regex::new(&pattern.pattern) {
                result = regex
                    .replace_all(&result, pattern.replacement.as_str())
                    .to_string();
            }
        }

        result
    }

    pub fn is_allowed_source(&self, source: &str) -> bool {
        !self.blocklist.contains(source)
            && (self.allowlist.is_empty() || self.allowlist.contains(source))
    }

    pub fn check_compliance(&self, source: &str, content: &str) -> ComplianceResult {
        if !self.is_allowed_source(source) {
            return ComplianceResult {
                compliant: false,
                issues: vec![ComplianceIssue {
                    severity: SensitivityLevel::Critical,
                    category: "blocked_source".to_string(),
                    message: format!("Source '{}' is blocked", source),
                }],
            };
        }

        let mut issues = Vec::new();
        let level = self.classify(content);

        if level == SensitivityLevel::Critical {
            issues.push(ComplianceIssue {
                severity: level.clone(),
                category: "sensitive_data".to_string(),
                message: "Critical sensitive data detected".to_string(),
            });
        }

        ComplianceResult {
            compliant: issues.is_empty(),
            issues,
        }
    }
}

impl Default for ContentClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceResult {
    pub compliant: bool,
    pub issues: Vec<ComplianceIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceIssue {
    pub severity: SensitivityLevel,
    pub category: String,
    pub message: String,
}
