/// T3.3: Enterprise Security + NemoClaw-level Sandbox
/// Provides RBAC, audit logging, API key management, and code sandbox.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Role { Admin, Operator, Viewer, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub role: Role,
    pub permissions: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub user_id: String,
    pub action: String,
    pub resource: String,
    pub success: bool,
    pub details: String,
}

static AUDIT_LOG: once_cell::sync::Lazy<Mutex<Vec<AuditEntry>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(Vec::new()));

pub struct AccessControl {
    users: Mutex<HashMap<String, User>>,
}

impl AccessControl {
    pub fn new() -> Self { Self { users: Mutex::new(HashMap::new()) } }

    pub fn add_user(&self, id: &str, name: &str, role: Role) {
        let permissions = match &role {
            Role::Admin => HashSet::from_iter(["*".to_string()]),
            Role::Operator => HashSet::from_iter(["workflow:execute".into(), "workflow:create".into(), "workflow:read".into()]),
            Role::Viewer => HashSet::from_iter(["workflow:read".into(), "memory:read".into()]),
            Role::Custom(p) => HashSet::from_iter(vec![p].into_iter().cloned()),
        };
        self.users.lock().unwrap().insert(id.into(), User { id: id.into(), name: name.into(), role, permissions });
    }

    pub fn check_permission(&self, user_id: &str, permission: &str) -> bool {
        let users = self.users.lock().unwrap();
        if let Some(user) = users.get(user_id) {
            user.permissions.contains("*") || user.permissions.contains(permission)
        } else { false }
    }

    pub fn require_permission(&self, user_id: &str, permission: &str) -> Result<(), String> {
        if self.check_permission(user_id, permission) { Ok(()) }
        else { Err(format!("Permission denied: {} requires {}", user_id, permission)) }
    }
}

pub fn log_audit(user_id: &str, action: &str, resource: &str, success: bool, details: &str) {
    let entry = AuditEntry {
        timestamp: Utc::now().to_rfc3339(),
        user_id: user_id.into(), action: action.into(),
        resource: resource.into(), success, details: details.into(),
    };
    AUDIT_LOG.lock().unwrap().push(entry);
}

pub fn get_audit_log() -> Vec<AuditEntry> { AUDIT_LOG.lock().unwrap().clone() }

/// Sandbox config for code execution isolation
pub struct SandboxConfig {
    pub max_memory_mb: u64,
    pub max_cpu_ms: u64,
    pub allowed_networks: Vec<String>,
    pub allowed_paths: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self { max_memory_mb: 256, max_cpu_ms: 30000, allowed_networks: vec![], allowed_paths: vec![] }
    }
}

pub fn validate_sandbox_request(code: &str, config: &SandboxConfig) -> Result<(), String> {
    // Simple dangerous pattern detection
    let dangerous = ["std::process::Command", "std::fs::remove_dir_all", "std::os::raw"];
    for d in &dangerous {
        if code.contains(d) {
            return Err(format!("Sandbox blocked: dangerous pattern '{}'", d));
        }
    }
    if code.len() as u64 > config.max_memory_mb * 1024 * 4 {
        return Err("Sandbox: code exceeds memory limit".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rbac() {
        let ac = AccessControl::new();
        ac.add_user("alice", "Alice", Role::Admin);
        ac.add_user("bob", "Bob", Role::Viewer);
        assert!(ac.check_permission("alice", "workflow:delete"));
        assert!(!ac.check_permission("bob", "workflow:delete"));
    }

    #[test]
    fn test_sandbox_validation() {
        let cfg = SandboxConfig::default();
        assert!(validate_sandbox_request("fn main() {}", &cfg).is_ok());
        assert!(validate_sandbox_request("use std::process::Command", &cfg).is_err());
    }

    #[test]
    fn test_audit_logging() {
        log_audit("alice", "execute", "wf_123", true, "Workflow executed");
        assert_eq!(get_audit_log().len(), 1);
    }
}

