use crate::Workflow;
use rusqlite::{Connection, Result as SqliteResult};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};

pub struct WorkflowDb {
    db_path: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionLog {
    pub id: String,
    pub workflow_id: String,
    pub version: u32,
    pub params: String,
    pub result: String,
    pub error: Option<String>,
    pub started_at: i64,
    pub finished_at: i64,
    pub duration_ms: i64,
}

impl WorkflowDb {
    pub fn open(path: &Path) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        Self::initialize_schema(&conn)?;
        Ok(Self {
            db_path: path.to_path_buf(),
        })
    }

    fn open_connection(&self) -> SqliteResult<Connection> {
        let conn = Connection::open(&self.db_path)?;
        // Enable WAL for better read concurrency; busy_timeout avoids SQLITE_BUSY on writes
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000;",
        )?;
        Ok(conn)
    }

    fn initialize_schema(conn: &Connection) -> SqliteResult<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS workflows (
                id TEXT NOT NULL,
                version INTEGER NOT NULL,
                name TEXT NOT NULL,
                n8n_json TEXT NOT NULL,
                ir_blob BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                is_latest INTEGER DEFAULT 1,
                PRIMARY KEY (id, version)
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_latest ON workflows(id, is_latest) WHERE is_latest=1",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS execution_logs (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                params TEXT NOT NULL,
                result TEXT,
                error TEXT,
                started_at INTEGER NOT NULL,
                finished_at INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_logs_workflow ON execution_logs(workflow_id, started_at)",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS workflow_diffs (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                from_version INTEGER NOT NULL,
                to_version INTEGER NOT NULL,
                diff_patch TEXT NOT NULL,
                user_id TEXT DEFAULT 'anonymous',
                created_at INTEGER NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_diffs_workflow ON workflow_diffs(workflow_id)",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS api_keys (
                key TEXT PRIMARY KEY,
                role TEXT NOT NULL DEFAULT 'Viewer',
                rate_limit INTEGER NOT NULL DEFAULT 60,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Learning core tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                owner TEXT,
                status TEXT NOT NULL DEFAULT 'created',
                checkpoint TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                started_at INTEGER,
                finished_at INTEGER
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tasks_workflow ON tasks(workflow_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS task_events (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                payload TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id)
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_task_events_task ON task_events(task_id, created_at)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS learning_units (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                unit_type TEXT NOT NULL,
                input_data TEXT NOT NULL,
                output_data TEXT,
                success BOOLEAN,
                duration_ms INTEGER,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_learning_units_workflow ON learning_units(workflow_id, created_at)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS learning_results (
                id TEXT PRIMARY KEY,
                unit_id TEXT NOT NULL,
                result_type TEXT NOT NULL,
                metrics TEXT NOT NULL,
                insights TEXT,
                confidence REAL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (unit_id) REFERENCES learning_units(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS learning_actions (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                action_type TEXT NOT NULL,
                params TEXT NOT NULL,
                applied BOOLEAN DEFAULT 0,
                created_at INTEGER NOT NULL,
                applied_at INTEGER
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_learning_actions_workflow ON learning_actions(workflow_id)",
            [],
        )?;

        Ok(())
    }

    pub fn save_workflow(
        &self,
        id: &str,
        name: &str,
        n8n_json: &JsonValue,
        ir: &Workflow,
    ) -> SqliteResult<u32> {
        let ir_blob = bincode::serialize(ir).unwrap_or_default();
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let conn = self.open_connection()?;

        let max_version: Option<u32> = conn
            .query_row(
                "SELECT MAX(version) FROM workflows WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .ok();
        let new_version = max_version.unwrap_or(0) + 1;

        conn.execute(
            "UPDATE workflows SET is_latest = 0 WHERE id = ?1 AND is_latest = 1",
            [id],
        )?;

        conn.execute(
            "INSERT INTO workflows (id, version, name, n8n_json, ir_blob, created_at, is_latest)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                id,
                new_version,
                name,
                &n8n_json.to_string(),
                &ir_blob,
                created_at,
                1,
            ),
        )?;
        Ok(new_version)
    }

    pub fn load_workflow(&self, id: &str, version: Option<u32>) -> SqliteResult<Option<Workflow>> {
        let conn = self.open_connection()?;
        let ir_blob: Option<Vec<u8>> = if let Some(ver) = version {
            let mut stmt =
                conn.prepare("SELECT ir_blob FROM workflows WHERE id = ?1 AND version = ?2")?;
            stmt.query_row([id, &ver.to_string()], |row| row.get(0))
                .ok()
        } else {
            let mut stmt =
                conn.prepare("SELECT ir_blob FROM workflows WHERE id = ?1 AND is_latest = 1")?;
            stmt.query_row([id], |row| row.get(0)).ok()
        };

        if let Some(blob) = ir_blob {
            let workflow: Workflow = bincode::deserialize(&blob).ok().unwrap_or_default();
            Ok(Some(workflow))
        } else {
            Ok(None)
        }
    }

    pub fn load_n8n_json(&self, id: &str, version: Option<u32>) -> SqliteResult<Option<String>> {
        let conn = self.open_connection()?;
        let json: Option<String> = if let Some(ver) = version {
            let mut stmt =
                conn.prepare("SELECT n8n_json FROM workflows WHERE id = ?1 AND version = ?2")?;
            stmt.query_row([id, &ver.to_string()], |row| row.get(0))
                .ok()
        } else {
            let mut stmt =
                conn.prepare("SELECT n8n_json FROM workflows WHERE id = ?1 AND is_latest = 1")?;
            stmt.query_row([id], |row| row.get(0)).ok()
        };

        Ok(json)
    }

    pub fn list_workflows(&self) -> SqliteResult<Vec<(String, String, u32)>> {
        let conn = self.open_connection()?;
        let mut stmt =
            conn.prepare("SELECT id, name, version FROM workflows WHERE is_latest = 1")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        rows.collect()
    }

    pub fn load_workflow_metadata(
        &self,
        id: &str,
        version: Option<u32>,
    ) -> SqliteResult<Option<(String, u32)>> {
        let conn = self.open_connection()?;
        if let Some(version) = version {
            let mut stmt =
                conn.prepare("SELECT name, version FROM workflows WHERE id = ?1 AND version = ?2")?;
            stmt.query_row(rusqlite::params![id, version], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .ok()
            .map_or(Ok(None), |row| Ok(Some(row)))
        } else {
            let mut stmt = conn
                .prepare("SELECT name, version FROM workflows WHERE id = ?1 AND is_latest = 1")?;
            stmt.query_row([id], |row| Ok((row.get(0)?, row.get(1)?)))
                .ok()
                .map_or(Ok(None), |row| Ok(Some(row)))
        }
    }

    pub fn list_versions(&self, id: &str) -> SqliteResult<Vec<u32>> {
        let conn = self.open_connection()?;
        let mut stmt =
            conn.prepare("SELECT version FROM workflows WHERE id = ?1 ORDER BY version DESC")?;
        let rows = stmt.query_map([id], |row| row.get(0))?;
        rows.collect()
    }

    pub fn rollback(&self, id: &str) -> SqliteResult<Option<u32>> {
        let conn = self.open_connection()?;

        conn.execute(
            "UPDATE workflows SET is_latest = 0 WHERE id = ?1 AND is_latest = 1",
            [id],
        )?;

        let prev_version: Option<u32> = conn
            .query_row(
                "SELECT MAX(version) FROM workflows WHERE id = ?1 AND version < (SELECT MAX(version) FROM workflows WHERE id = ?1)",
                [id],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        if let Some(ver) = prev_version {
            conn.execute(
                "UPDATE workflows SET is_latest = 1 WHERE id = ?1 AND version = ?2",
                rusqlite::params![id, &ver.to_string()],
            )?;
            Ok(Some(ver))
        } else {
            Ok(None)
        }
    }

    pub fn save_execution_log(&self, log: &ExecutionLog) -> SqliteResult<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO execution_logs (id, workflow_id, version, params, result, error, started_at, finished_at, duration_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                log.id,
                log.workflow_id,
                log.version,
                log.params,
                log.result,
                log.error,
                log.started_at,
                log.finished_at,
                log.duration_ms,
            ],
        )?;
        Ok(())
    }

    pub fn get_execution_logs(
        &self,
        workflow_id: &str,
        limit: usize,
    ) -> SqliteResult<Vec<ExecutionLog>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, workflow_id, version, params, result, error, started_at, finished_at, duration_ms FROM execution_logs WHERE workflow_id = ?1 ORDER BY started_at DESC LIMIT ?2"
        )?;
        let logs = stmt.query_map(rusqlite::params![workflow_id, limit as i64], |row| {
            Ok(ExecutionLog {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                version: row.get(2)?,
                params: row.get(3)?,
                result: row.get(4)?,
                error: row.get(5)?,
                started_at: row.get(6)?,
                finished_at: row.get(7)?,
                duration_ms: row.get(8)?,
            })
        })?;
        logs.collect()
    }

    pub fn get_recent_logs(&self, limit: usize) -> SqliteResult<Vec<ExecutionLog>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, workflow_id, version, params, result, error, started_at, finished_at, duration_ms FROM execution_logs ORDER BY started_at DESC LIMIT ?1"
        )?;
        let logs = stmt.query_map([limit as i64], |row| {
            Ok(ExecutionLog {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                version: row.get(2)?,
                params: row.get(3)?,
                result: row.get(4)?,
                error: row.get(5)?,
                started_at: row.get(6)?,
                finished_at: row.get(7)?,
                duration_ms: row.get(8)?,
            })
        })?;
        logs.collect()
    }

    pub fn save_workflow_diff(
        &self,
        id: &str,
        workflow_id: &str,
        from_version: u32,
        to_version: u32,
        diff_patch: &str,
        user_id: &str,
    ) -> SqliteResult<()> {
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO workflow_diffs (id, workflow_id, from_version, to_version, diff_patch, user_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, workflow_id, from_version, to_version, diff_patch, user_id, created_at],
        )?;
        Ok(())
    }

    pub fn get_workflow_diffs(
        &self,
        workflow_id: &str,
    ) -> SqliteResult<Vec<(String, String, u32, u32, String, String, i64)>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, workflow_id, from_version, to_version, diff_patch, user_id, created_at FROM workflow_diffs WHERE workflow_id = ?1 ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([workflow_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })?;
        rows.collect()
    }

    pub fn save_api_key(
        &self,
        key: &str,
        role: &str,
        rate_limit: u32,
        created_at: i64,
    ) -> SqliteResult<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO api_keys (key, role, rate_limit, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![key, role, rate_limit, created_at],
        )?;
        Ok(())
    }

    pub fn load_api_keys(&self) -> SqliteResult<Vec<(String, String, u32, i64)>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare("SELECT key, role, rate_limit, created_at FROM api_keys")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;
        rows.collect()
    }

    pub fn delete_api_key(&self, key: &str) -> SqliteResult<()> {
        let conn = self.open_connection()?;
        conn.execute("DELETE FROM api_keys WHERE key = ?1", [key])?;
        Ok(())
    }

    pub fn get_stats(&self) -> SqliteResult<DbStats> {
        let conn = self.open_connection()?;

        let total_workflows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM workflows WHERE is_latest = 1",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let total_executions: i64 = conn
            .query_row("SELECT COUNT(*) FROM execution_logs", [], |r| r.get(0))
            .unwrap_or(0);

        let successful_executions: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM execution_logs WHERE error IS NULL OR error = ''",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let avg_duration_ms: f64 = conn
            .query_row("SELECT AVG(duration_ms) FROM execution_logs", [], |r| {
                r.get(0)
            })
            .unwrap_or(0.0);

        let executions_last_24h: i64 = {
            let cutoff = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64)
                - 86400;
            conn.query_row(
                "SELECT COUNT(*) FROM execution_logs WHERE started_at >= ?1",
                [cutoff],
                |r| r.get(0),
            )
            .unwrap_or(0)
        };

        let success_rate = if total_executions > 0 {
            successful_executions as f64 / total_executions as f64 * 100.0
        } else {
            0.0
        };

        Ok(DbStats {
            total_workflows,
            total_executions,
            successful_executions,
            failed_executions: total_executions - successful_executions,
            success_rate,
            avg_duration_ms,
            executions_last_24h,
        })
    }
}

#[derive(Debug, serde::Serialize)]
pub struct DbStats {
    pub total_workflows: i64,
    pub total_executions: i64,
    pub successful_executions: i64,
    pub failed_executions: i64,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub executions_last_24h: i64,
}

// ─── Learning patterns & insights ────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
pub struct WorkflowPattern {
    pub workflow_id: String,
    pub workflow_name: String,
    pub total_executions: i64,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub last_executed_at: i64,
    pub trend: String, // "improving" | "degrading" | "stable"
}

#[derive(Debug, serde::Serialize)]
pub struct WorkflowInsight {
    pub insight_type: String, // "performance" | "reliability" | "optimization" | "anomaly"
    pub workflow_id: String,
    pub workflow_name: String,
    pub title: String,
    pub description: String,
    pub confidence: f64,
    pub recommendation: String,
}

impl WorkflowDb {
    /// 分析执行日志，生成每个工作流的统计模式
    pub fn get_patterns(&self) -> SqliteResult<Vec<WorkflowPattern>> {
        let conn = self.open_connection()?;

        // 计算每个工作流的执行统计
        let mut stmt = conn.prepare(
            r#"SELECT
                el.workflow_id,
                COALESCE(w.name, el.workflow_id) AS workflow_name,
                COUNT(*) AS total_executions,
                SUM(CASE WHEN el.error IS NULL OR el.error = '' THEN 1 ELSE 0 END) AS successes,
                AVG(el.duration_ms) AS avg_duration,
                MAX(el.started_at) AS last_exec
               FROM execution_logs el
               LEFT JOIN workflows w ON w.id = el.workflow_id AND w.is_latest = 1
               GROUP BY el.workflow_id
               HAVING total_executions >= 1
               ORDER BY total_executions DESC
               LIMIT 50"#,
        )?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let cutoff_7d = now - 7 * 86400;

        let patterns = stmt
            .query_map([], |row| {
                let workflow_id: String = row.get(0)?;
                let workflow_name: String = row.get(1)?;
                let total: i64 = row.get(2)?;
                let successes: i64 = row.get(3).unwrap_or(0);
                let avg_dur: f64 = row.get(4).unwrap_or(0.0);
                let last_exec: i64 = row.get(5).unwrap_or(0);

                let success_rate = if total > 0 {
                    successes as f64 / total as f64 * 100.0
                } else {
                    0.0
                };

                Ok((workflow_id, workflow_name, total, success_rate, avg_dur, last_exec))
            })?
            .filter_map(|r| r.ok())
            .map(|(workflow_id, workflow_name, total, success_rate, avg_dur, last_exec)| {
                // 趋势计算：对比近7天 vs 全部的成功率
                let trend_query = conn.query_row(
                    r#"SELECT
                        SUM(CASE WHEN error IS NULL OR error = '' THEN 1 ELSE 0 END) * 1.0 / COUNT(*)
                       FROM execution_logs
                       WHERE workflow_id = ?1 AND started_at >= ?2"#,
                    rusqlite::params![&workflow_id, cutoff_7d],
                    |r| r.get::<_, f64>(0),
                ).unwrap_or(success_rate / 100.0);

                let trend = if trend_query * 100.0 > success_rate + 5.0 {
                    "improving"
                } else if trend_query * 100.0 < success_rate - 5.0 {
                    "degrading"
                } else {
                    "stable"
                };

                WorkflowPattern {
                    workflow_id,
                    workflow_name,
                    total_executions: total,
                    success_rate: (success_rate * 10.0).round() / 10.0,
                    avg_duration_ms: (avg_dur * 10.0).round() / 10.0,
                    last_executed_at: last_exec,
                    trend: trend.to_string(),
                }
            })
            .collect();

        Ok(patterns)
    }

    /// 基于执行统计生成优化建议
    pub fn get_insights(&self) -> SqliteResult<Vec<WorkflowInsight>> {
        let patterns = self.get_patterns()?;
        let mut insights = Vec::new();

        for p in &patterns {
            // 低成功率洞察
            if p.success_rate < 70.0 && p.total_executions >= 3 {
                insights.push(WorkflowInsight {
                    insight_type: "reliability".to_string(),
                    workflow_id: p.workflow_id.clone(),
                    workflow_name: p.workflow_name.clone(),
                    title: format!("成功率偏低 ({:.1}%)", p.success_rate),
                    description: format!(
                        "工作流 {} 在最近 {} 次执行中成功率仅 {:.1}%，低于阈值 70%。",
                        p.workflow_name, p.total_executions, p.success_rate
                    ),
                    confidence: 0.9,
                    recommendation: "检查节点参数配置、API 密钥有效性，以及外部依赖可用性。建议添加重试节点和错误处理分支。".to_string(),
                });
            }

            // 高延迟洞察
            if p.avg_duration_ms > 10000.0 && p.total_executions >= 2 {
                insights.push(WorkflowInsight {
                    insight_type: "performance".to_string(),
                    workflow_id: p.workflow_id.clone(),
                    workflow_name: p.workflow_name.clone(),
                    title: format!("执行耗时过长 ({:.0}ms 均值)", p.avg_duration_ms),
                    description: format!(
                        "工作流 {} 平均执行时间 {:.0}ms，可能存在性能瓶颈。",
                        p.workflow_name, p.avg_duration_ms
                    ),
                    confidence: 0.8,
                    recommendation: "考虑将串行 HTTP 请求改为并行执行，或添加缓存节点减少重复请求。".to_string(),
                });
            }

            // 趋势恶化洞察
            if p.trend == "degrading" {
                insights.push(WorkflowInsight {
                    insight_type: "anomaly".to_string(),
                    workflow_id: p.workflow_id.clone(),
                    workflow_name: p.workflow_name.clone(),
                    title: "近期性能下降趋势".to_string(),
                    description: format!(
                        "工作流 {} 近7天成功率相比历史水平有明显下降。",
                        p.workflow_name
                    ),
                    confidence: 0.75,
                    recommendation:
                        "检查近期变更，对比最新版本与历史版本的执行日志，考虑回滚到稳定版本。"
                            .to_string(),
                });
            }

            // 优化建议：高频工作流
            if p.total_executions >= 20 && p.success_rate >= 90.0 {
                insights.push(WorkflowInsight {
                    insight_type: "optimization".to_string(),
                    workflow_id: p.workflow_id.clone(),
                    workflow_name: p.workflow_name.clone(),
                    title: format!("高频稳定工作流 ({}次)", p.total_executions),
                    description: format!(
                        "工作流 {} 已执行 {} 次，成功率 {:.1}%，运行稳定。",
                        p.workflow_name, p.total_executions, p.success_rate
                    ),
                    confidence: 0.95,
                    recommendation: "该工作流可纳入自动学习优化队列，系统将自动调整参数以提升效率。".to_string(),
                });
            }
        }

        // 全局洞察
        let conn = self.open_connection()?;
        let unique_errors: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT error) FROM execution_logs WHERE error IS NOT NULL AND error != '' LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if unique_errors > 5 {
            insights.push(WorkflowInsight {
                insight_type: "reliability".to_string(),
                workflow_id: "global".to_string(),
                workflow_name: "全局".to_string(),
                title: format!("系统存在 {} 种不同错误模式", unique_errors),
                description: "多种错误类型在不同工作流中出现，建议统一错误处理策略。".to_string(),
                confidence: 0.7,
                recommendation: "为所有工作流添加统一的错误捕获节点，并配置告警通知。".to_string(),
            });
        }

        Ok(insights)
    }
}

// ─── Task State Machine ──────────────────────────────────────────────

const VALID_TRANSITIONS: &[&[&str]] = &[
    &["created", "running"],
    &["created", "blocked"],
    &["created", "failed"],
    &["running", "blocked"],
    &["running", "review"],
    &["running", "done"],
    &["running", "failed"],
    &["blocked", "running"],
    &["blocked", "failed"],
    &["review", "done"],
    &["review", "failed"],
    &["review", "running"],
];

pub fn validate_state_transition(from: &str, to: &str) -> bool {
    VALID_TRANSITIONS.iter().any(|t| t[0] == from && t[1] == to)
}

pub fn get_valid_transitions(from: &str) -> Vec<&'static str> {
    VALID_TRANSITIONS
        .iter()
        .filter(|t| t[0] == from)
        .map(|t| t[1])
        .collect()
}

// ─── Task Management ─────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub id: String,
    pub workflow_id: String,
    pub owner: Option<String>,
    pub status: String,
    pub checkpoint: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskEvent {
    pub id: String,
    pub task_id: String,
    pub event_type: String,
    pub payload: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LearningUnit {
    pub id: String,
    pub workflow_id: String,
    pub unit_type: String,
    pub input_data: String,
    pub output_data: Option<String>,
    pub success: Option<bool>,
    pub duration_ms: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LearningAction {
    pub id: String,
    pub workflow_id: String,
    pub action_type: String,
    pub params: String,
    pub applied: bool,
    pub created_at: i64,
    pub applied_at: Option<i64>,
}

impl WorkflowDb {
    pub fn create_task(
        &self,
        id: &str,
        workflow_id: &str,
        owner: Option<&str>,
        checkpoint: Option<&str>,
    ) -> SqliteResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO tasks (id, workflow_id, owner, status, checkpoint, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, workflow_id, owner, "created", checkpoint, now, now],
        )?;
        Ok(())
    }

    pub fn update_task_status(
        &self,
        task_id: &str,
        status: &str,
        checkpoint: Option<&str>,
    ) -> SqliteResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let conn = self.open_connection()?;
        let started: Option<i64> = conn
            .query_row(
                "SELECT started_at FROM tasks WHERE id = ?1",
                [task_id],
                |r| r.get(0),
            )
            .ok();

        let started_at = if status == "running" && started.is_none() {
            Some(now)
        } else {
            started
        };

        let finished_at = if status == "done" || status == "failed" {
            Some(now)
        } else {
            None
        };

        conn.execute(
            "UPDATE tasks SET status = ?1, checkpoint = COALESCE(?2, checkpoint), updated_at = ?3, started_at = COALESCE(started_at, ?4), finished_at = ?5 WHERE id = ?6",
            rusqlite::params![status, checkpoint, now, started_at, finished_at, task_id],
        )?;
        Ok(())
    }

    pub fn update_task_status_validated(
        &self,
        task_id: &str,
        new_status: &str,
        checkpoint: Option<&str>,
    ) -> SqliteResult<bool> {
        if let Some(task) = self.get_task(task_id)? {
            if !validate_state_transition(&task.status, new_status) {
                return Ok(false);
            }
            self.update_task_status(task_id, new_status, checkpoint)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn get_task(&self, task_id: &str) -> SqliteResult<Option<Task>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, workflow_id, owner, status, checkpoint, created_at, updated_at, started_at, finished_at FROM tasks WHERE id = ?1"
        )?;
        let task = stmt
            .query_row([task_id], |row| {
                Ok(Task {
                    id: row.get(0)?,
                    workflow_id: row.get(1)?,
                    owner: row.get(2)?,
                    status: row.get(3)?,
                    checkpoint: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    started_at: row.get(7)?,
                    finished_at: row.get(8)?,
                })
            })
            .ok();
        Ok(task)
    }

    pub fn list_tasks(
        &self,
        workflow_id: Option<&str>,
        status: Option<&str>,
        limit: usize,
    ) -> SqliteResult<Vec<Task>> {
        let conn = self.open_connection()?;
        let query = match (workflow_id, status) {
            (Some(wf), Some(st)) => format!(
                "SELECT id, workflow_id, owner, status, checkpoint, created_at, updated_at, started_at, finished_at FROM tasks WHERE workflow_id = '{}' AND status = '{}' ORDER BY created_at DESC LIMIT {}",
                wf, st, limit
            ),
            (Some(wf), None) => format!(
                "SELECT id, workflow_id, owner, status, checkpoint, created_at, updated_at, started_at, finished_at FROM tasks WHERE workflow_id = '{}' ORDER BY created_at DESC LIMIT {}",
                wf, limit
            ),
            (None, Some(st)) => format!(
                "SELECT id, workflow_id, owner, status, checkpoint, created_at, updated_at, started_at, finished_at FROM tasks WHERE status = '{}' ORDER BY created_at DESC LIMIT {}",
                st, limit
            ),
            _ => format!(
                "SELECT id, workflow_id, owner, status, checkpoint, created_at, updated_at, started_at, finished_at FROM tasks ORDER BY created_at DESC LIMIT {}",
                limit
            ),
        };
        let mut stmt = conn.prepare(&query)?;
        let tasks = stmt.query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                owner: row.get(2)?,
                status: row.get(3)?,
                checkpoint: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                started_at: row.get(7)?,
                finished_at: row.get(8)?,
            })
        })?;
        tasks.collect()
    }

    pub fn add_task_event(
        &self,
        id: &str,
        task_id: &str,
        event_type: &str,
        payload: Option<&str>,
    ) -> SqliteResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO task_events (id, task_id, event_type, payload, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, task_id, event_type, payload, now],
        )?;
        Ok(())
    }

    pub fn get_task_events(&self, task_id: &str) -> SqliteResult<Vec<TaskEvent>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, task_id, event_type, payload, created_at FROM task_events WHERE task_id = ?1 ORDER BY created_at ASC"
        )?;
        let events = stmt.query_map([task_id], |row| {
            Ok(TaskEvent {
                id: row.get(0)?,
                task_id: row.get(1)?,
                event_type: row.get(2)?,
                payload: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        events.collect()
    }

    pub fn save_learning_unit(&self, unit: &LearningUnit) -> SqliteResult<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO learning_units (id, workflow_id, unit_type, input_data, output_data, success, duration_ms, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![unit.id, unit.workflow_id, unit.unit_type, unit.input_data, unit.output_data, unit.success, unit.duration_ms, unit.created_at],
        )?;
        Ok(())
    }

    pub fn get_learning_units(
        &self,
        workflow_id: &str,
        limit: usize,
    ) -> SqliteResult<Vec<LearningUnit>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, workflow_id, unit_type, input_data, output_data, success, duration_ms, created_at FROM learning_units WHERE workflow_id = ?1 ORDER BY created_at DESC LIMIT ?2"
        )?;
        let units = stmt.query_map(rusqlite::params![workflow_id, limit as i64], |row| {
            Ok(LearningUnit {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                unit_type: row.get(2)?,
                input_data: row.get(3)?,
                output_data: row.get(4)?,
                success: row.get(5)?,
                duration_ms: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        units.collect()
    }

    pub fn save_learning_action(&self, action: &LearningAction) -> SqliteResult<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO learning_actions (id, workflow_id, action_type, params, applied, created_at, applied_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![action.id, action.workflow_id, action.action_type, action.params, action.applied as i32, action.created_at, action.applied_at],
        )?;
        Ok(())
    }

    pub fn apply_learning_action(&self, action_id: &str) -> SqliteResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let conn = self.open_connection()?;
        conn.execute(
            "UPDATE learning_actions SET applied = 1, applied_at = ?1 WHERE id = ?2",
            rusqlite::params![now, action_id],
        )?;
        Ok(())
    }

    pub fn get_pending_actions(&self, workflow_id: &str) -> SqliteResult<Vec<LearningAction>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, workflow_id, action_type, params, applied, created_at, applied_at FROM learning_actions WHERE workflow_id = ?1 AND applied = 0 ORDER BY created_at ASC"
        )?;
        let actions = stmt.query_map([workflow_id], |row| {
            Ok(LearningAction {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                action_type: row.get(2)?,
                params: row.get(3)?,
                applied: row.get::<_, i32>(4)? == 1,
                created_at: row.get(5)?,
                applied_at: row.get(6)?,
            })
        })?;
        actions.collect()
    }
}
