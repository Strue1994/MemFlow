/// B2: Sandbox Execution Environment
///
/// Upgrades the existing security.rs sandbox with:
/// - Sandbox trait (LocalSandbox, DockerSandbox)
/// - Rule engine for command whitelisting
/// - Output capture and size limiting
/// - Integration with executor's workflow execution

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::Instant;
use std::process::Stdio;

// ---- Types ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub mode: SandboxMode,
    pub memory_limit_mb: u64,
    pub time_limit_secs: u64,
    pub output_limit_bytes: u64,
    pub allowed_commands: Vec<String>,
    pub blocked_commands: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub network_access: bool,
    pub env_vars: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SandboxMode {
    /// Direct execution on host (development only)
    Local,
    /// Docker container execution
    Docker,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            mode: SandboxMode::Local,
            memory_limit_mb: 256,
            time_limit_secs: 30,
            output_limit_bytes: 1_000_000, // 1MB
            allowed_commands: vec![
                "ls".into(), "cat".into(), "echo".into(), "pwd".into(),
                "node".into(), "python".into(), "python3".into(),
                "cargo".into(), "rustc".into(), "npm".into(), "npx".into(),
            ],
            blocked_commands: vec![
                "rm".into(), "sudo".into(), "docker".into(), "dd".into(),
                "mkfs".into(), "fdisk".into(), "mount".into(), "chmod".into(),
                "chown".into(), "kill".into(), "pkill".into(),
            ],
            allowed_paths: vec![],
            network_access: false,
            env_vars: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub truncated: bool,
    pub blocked: bool,
    pub block_reason: Option<String>,
}

// ---- Rule Engine ----

#[derive(Debug)]
pub struct RuleEngine {
    config: Arc<RwLock<SandboxConfig>>,
}

impl RuleEngine {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config: Arc::new(RwLock::new(config)) }
    }

    pub async fn check_command(&self, command: &str) -> Result<(), String> {
        let cfg = self.config.read().await;

        // Extract the base command (first word)
        let base_cmd = command.split_whitespace().next().unwrap_or("");

        if base_cmd.is_empty() {
            return Err("Empty command".into());
        }

        // Check blocked list first
        for blocked in &cfg.blocked_commands {
            if base_cmd == blocked || command.contains(blocked) {
                return Err(format!("Command '{}' is blocked by sandbox policy", base_cmd));
            }
        }

        // If allowed_commands is non-empty, enforce whitelist
        if !cfg.allowed_commands.is_empty() {
            let allowed = cfg.allowed_commands.iter().any(|a| base_cmd == a);
            if !allowed {
                return Err(format!(
                    "Command '{}' is not in the allowed list. Allowed: {:?}",
                    base_cmd, cfg.allowed_commands
                ));
            }
        }

        Ok(())
    }

    pub async fn update_config(&self, config: SandboxConfig) {
        *self.config.write().await = config;
    }

    pub async fn get_config(&self) -> SandboxConfig {
        self.config.read().await.clone()
    }
}

// ---- Sandbox Trait (simplified) ----

pub trait Sandbox: Send + Sync {
    fn config(&self) -> &Arc<RwLock<SandboxConfig>>;
    fn mode(&self) -> SandboxMode;
}

impl Sandbox for LocalSandbox {
    fn config(&self) -> &Arc<RwLock<SandboxConfig>> { &self.config }
    fn mode(&self) -> SandboxMode { SandboxMode::Local }
}

impl Sandbox for DockerSandbox {
    fn config(&self) -> &Arc<RwLock<SandboxConfig>> { &self.config }
    fn mode(&self) -> SandboxMode { SandboxMode::Docker }
}

// ---- Local Sandbox ----

pub struct LocalSandbox {
    pub config: Arc<RwLock<SandboxConfig>>,
    rule_engine: Arc<RuleEngine>,
}

impl LocalSandbox {
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config.clone())),
            rule_engine: Arc::new(RuleEngine::new(config)),
        }
    }

    pub async fn execute(&self, command: &str, args: &[&str]) -> SandboxResult {
        let start = Instant::now();

        // Build full command string for rule check
        let full_cmd = if args.is_empty() {
            command.to_string()
        } else {
            format!("{} {}", command, args.join(" "))
        };

        // Rule check
        if let Err(reason) = self.rule_engine.check_command(&full_cmd).await {
            return SandboxResult {
                success: false, stdout: String::new(), stderr: String::new(),
                exit_code: -1, duration_ms: start.elapsed().as_millis() as u64,
                truncated: false, blocked: true, block_reason: Some(reason),
            };
        }

        let cfg = self.config.read().await.clone();
        drop(cfg); // Release read lock before async execution

        // Execute via tokio::process
        let output = tokio::process::Command::new(command)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output()
            .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match output {
            Ok(out) => {
                let stdout_str = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr_str = String::from_utf8_lossy(&out.stderr).to_string();

                let cfg = self.config.read().await;
                let (truncated, final_stdout) = if stdout_str.len() > cfg.output_limit_bytes as usize {
                    (true, stdout_str[..cfg.output_limit_bytes as usize].to_string())
                } else {
                    (false, stdout_str)
                };

                SandboxResult {
                    success: out.status.success(),
                    stdout: final_stdout,
                    stderr: stderr_str,
                    exit_code: out.status.code().unwrap_or(-1),
                    duration_ms,
                    truncated,
                    blocked: false,
                    block_reason: None,
                }
            }
            Err(e) => SandboxResult {
                success: false, stdout: String::new(),
                stderr: format!("Execution error: {}", e),
                exit_code: -1, duration_ms,
                truncated: false, blocked: false, block_reason: None,
            },
        }
    }
}

// ---- Docker Sandbox (future) ----

pub struct DockerSandbox {
    config: Arc<RwLock<SandboxConfig>>,
}

impl DockerSandbox {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config: Arc::new(RwLock::new(config)) }
    }
}

pub fn create_docker_sandbox(config: SandboxConfig) -> DockerSandbox {
    DockerSandbox::new(config)
}

// ---- Sandbox Manager ----

pub struct SandboxManager {
    local: LocalSandbox,
    rule_engine: Arc<RuleEngine>,
}

impl SandboxManager {
    pub fn new_local() -> Self {
        let config = SandboxConfig::default();
        let rule_engine = Arc::new(RuleEngine::new(config.clone()));
        let local = LocalSandbox::new(config);
        Self { local, rule_engine }
    }

    pub async fn execute(&self, command: &str, args: &[&str]) -> SandboxResult {
        self.local.execute(command, args).await
    }

    pub async fn update_config(&self, config: SandboxConfig) {
        self.rule_engine.update_config(config.clone()).await;
        // LocalSandbox config is updated through the shared Arc<RwLock<>>
        // The inner sandbox instance stays the same with refreshed rule engine
        let mut locked = self.local.config.write().await;
        *locked = config;
    }

    pub async fn get_config(&self) -> SandboxConfig {
        self.rule_engine.get_config().await
    }

    pub fn rule_engine(&self) -> &Arc<RuleEngine> {
        &self.rule_engine
    }
}

// ---- Global instance ----

use once_cell::sync::Lazy;
pub static GLOBAL_SANDBOX: Lazy<SandboxManager> = Lazy::new(|| {
    SandboxManager::new_local()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_sandbox_echo() {
        let sandbox = LocalSandbox::new(SandboxConfig::default());
        #[cfg(windows)]
        let result = sandbox.execute("cmd", &["/c", "echo", "hello world"]).await;
        #[cfg(not(windows))]
        let result = sandbox.execute("echo", &["hello world"]).await;
        assert!(result.success, "Expected success, got exit_code={} stderr={:?}", result.exit_code, result.stderr);
        assert!(result.stdout.contains("hello world"), "stdout: {:?}", result.stdout);
    }

    #[tokio::test]
    async fn test_output_truncation() {
        let mut cfg = SandboxConfig::default();
        cfg.output_limit_bytes = 10;
        let sandbox = LocalSandbox::new(cfg);
        #[cfg(windows)]
        let result = sandbox.execute("cmd", &["/c", "echo", "a b c d e f g"]).await;
        #[cfg(not(windows))]
        let result = sandbox.execute("echo", &["a b c d e f g"]).await;
        assert!(result.truncated, "Expected truncation, got stdout length={}", result.stdout.len());
    }

    #[tokio::test]
    async fn test_rule_engine_blocks_dangerous() {
        let engine = RuleEngine::new(SandboxConfig::default());
        let result = engine.check_command("rm -rf /").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("blocked"));
    }

    #[tokio::test]
    async fn test_rule_engine_allows_known() {
        let engine = RuleEngine::new(SandboxConfig::default());
        let result = engine.check_command("echo test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_output_truncation() {
        let mut cfg = SandboxConfig::default();
        cfg.output_limit_bytes = 10;
        let sandbox = LocalSandbox::new(cfg);
        let result = sandbox.execute("echo", &["a b c d e f g"]).await;
        assert!(result.truncated);
    }
}
