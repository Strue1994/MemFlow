use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{Utc, Duration};
use rusqlite::{Connection, params};

pub struct ExecutionLogger {
    conn: Arc<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub workflow_id: String,
    pub version: u32,
    pub params: String,
    pub result: String,
    pub error: Option<String>,
    pub started_at: i64,
    pub finished_at: i64,
    pub duration_ms: i64,
    pub variant: Option<String>,
}

impl ExecutionLogger {
    pub fn new(db_path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS execution_logs (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                params TEXT NOT NULL,
                result TEXT NOT NULL,
                error TEXT,
                started_at INTEGER NOT NULL,
                finished_at INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                variant TEXT,
                created_at INTEGER DEFAULT (strftime('%s', 'now'))
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_workflow_id ON execution_logs(workflow_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_variant ON execution_logs(variant)",
            [],
        )?;

        Ok(Self { conn: Arc::new(conn) })
    }

    pub fn log_execution(&self, entry: &LogEntry) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO execution_logs (id, workflow_id, version, params, result, error, started_at, finished_at, duration_ms, variant)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                entry.id,
                entry.workflow_id,
                entry.version,
                entry.params,
                entry.result,
                entry.error,
                entry.started_at,
                entry.finished_at,
                entry.duration_ms,
                entry.variant,
            ],
        )?;
        Ok(())
    }

    pub fn get_logs(&self, workflow_id: &str, limit: usize) -> Result<Vec<LogEntry>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, workflow_id, version, params, result, error, started_at, finished_at, duration_ms, variant
             FROM execution_logs WHERE workflow_id = ?1 ORDER BY started_at DESC LIMIT ?2"
        )?;
        
        let entries = stmt.query_map(params![workflow_id, limit as i64], |row| {
            Ok(LogEntry {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                version: row.get(2)?,
                params: row.get(3)?,
                result: row.get(4)?,
                error: row.get(5)?,
                started_at: row.get(6)?,
                finished_at: row.get(7)?,
                duration_ms: row.get(8)?,
                variant: row.get(9)?,
            })
        })?;

        entries.collect()
    }

    pub fn get_variant_logs(&self, workflow_id: &str, variant: &str) -> Result<Vec<LogEntry>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, workflow_id, version, params, result, error, started_at, finished_at, duration_ms, variant
             FROM execution_logs WHERE workflow_id = ?1 AND variant = ?2 ORDER BY started_at DESC"
        )?;
        
        let entries = stmt.query_map(params![workflow_id, variant], |row| {
            Ok(LogEntry {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                version: row.get(2)?,
                params: row.get(3)?,
                result: row.get(4)?,
                error: row.get(5)?,
                started_at: row.get(6)?,
                finished_at: row.get(7)?,
                duration_ms: row.get(8)?,
                variant: row.get(9)?,
            })
        })?;

        entries.collect()
    }

    pub fn get_all_workflows(&self) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT DISTINCT workflow_id FROM execution_logs")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect()
    }

    pub fn cleanup_old_logs(&self, days: u32) -> Result<usize, rusqlite::Error> {
        let cutoff = Utc::now() - Duration::days(days as i64);
        let count = self.conn.execute(
            "DELETE FROM execution_logs WHERE started_at < ?1",
            params![cutoff.timestamp()],
        )?;
        Ok(count)
    }
}

pub struct ABTestManager {
    logger: Arc<ExecutionLogger>,
    active_tests: Arc<RwLock<HashMap<String, ABTestConfig>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestConfig {
    pub test_id: String,
    pub workflow_id: String,
    pub variant_a: String,
    pub variant_b: String,
    pub traffic_split: f32,
    pub start_time: i64,
    pub min_samples: u32,
}

impl ABTestManager {
    pub fn new(logger: Arc<ExecutionLogger>) -> Self {
        Self {
            logger,
            active_tests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_test(&self, config: ABTestConfig) {
        let mut tests = self.active_tests.write().await;
        tests.insert(config.workflow_id.clone(), config);
    }

    pub async fn get_variant(&self, workflow_id: &str) -> Option<String> {
        let tests = self.active_tests.read().await;
        let test = tests.get(workflow_id)?;
        
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u32;
        
        if (seed % 1000) as f32 / 1000.0 < test.traffic_split {
            Some("B".to_string())
        } else {
            Some("A".to_string())
        }
    }

    pub async fn evaluate_test(&self, workflow_id: &str) -> Option<ABTestEvalResult> {
        let tests = self.active_tests.read().await;
        let test = tests.get(workflow_id)?;
        
        let logs_a = self.logger.get_variant_logs(workflow_id, "A").ok()?;
        let logs_b = self.logger.get_variant_logs(workflow_id, "B").ok()?;
        
        if logs_a.len() < test.min_samples as usize || logs_b.len() < test.min_samples as usize {
            return None;
        }

        let success_a = logs_a.iter().filter(|l| l.error.is_none()).count();
        let success_b = logs_b.iter().filter(|l| l.error.is_none()).count();
        
        let rate_a = success_a as f64 / logs_a.len() as f64;
        let rate_b = success_b as f64 / logs_b.len() as f64;
        
        let avg_dur_a = logs_a.iter().map(|l| l.duration_ms).sum::<i64>() as f64 / logs_a.len() as f64;
        let avg_dur_b = logs_b.iter().map(|l| l.duration_ms).sum::<i64>() as f64 / logs_b.len() as f64;

        let winner = if rate_a > rate_b {
            Some("A".to_string())
        } else if rate_b > rate_a {
            Some("B".to_string())
        } else if avg_dur_a < avg_dur_b {
            Some("A".to_string())
        } else if avg_dur_b < avg_dur_a {
            Some("B".to_string())
        } else {
            None
        };

        Some(ABTestEvalResult {
            test_id: test.test_id.clone(),
            variant_a_samples: logs_a.len() as u32,
            variant_b_samples: logs_b.len() as u32,
            variant_a_success_rate: rate_a,
            variant_b_success_rate: rate_b,
            variant_a_avg_duration_ms: avg_dur_a,
            variant_b_avg_duration_ms: avg_dur_b,
            winner,
            confidence: (logs_a.len().min(logs_b.len()) as f32 / (logs_a.len() + logs_b.len()) as f32).min(0.95),
        })
    }

    pub async fn stop_test(&self, workflow_id: &str) -> Option<ABTestEvalResult> {
        let result = self.evaluate_test(workflow_id).await;
        let mut tests = self.active_tests.write().await;
        tests.remove(workflow_id);
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestEvalResult {
    pub test_id: String,
    pub variant_a_samples: u32,
    pub variant_b_samples: u32,
    pub variant_a_success_rate: f64,
    pub variant_b_success_rate: f64,
    pub variant_a_avg_duration_ms: f64,
    pub variant_b_avg_duration_ms: f64,
    pub winner: Option<String>,
    pub confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_creation() {
        let logger = ExecutionLogger::new(":memory:").unwrap();
        assert!(logger.get_all_workflows().unwrap().is_empty());
    }

    #[test]
    fn test_log_entry() {
        let logger = ExecutionLogger::new(":memory:").unwrap();
        let entry = LogEntry {
            id: "test_1".to_string(),
            workflow_id: "wf1".to_string(),
            version: 1,
            params: "{}".to_string(),
            result: "{}".to_string(),
            error: None,
            started_at: 1000,
            finished_at: 1100,
            duration_ms: 100,
            variant: Some("A".to_string()),
        };
        logger.log_execution(&entry).unwrap();
        
        let logs = logger.get_logs("wf1", 10).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].variant, Some("A".to_string()));
    }
}