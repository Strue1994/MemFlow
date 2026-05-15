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
        Connection::open(&self.db_path)
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
            stmt.query_row(rusqlite::params![id, version], |row| Ok((row.get(0)?, row.get(1)?)))
                .ok()
                .map_or(Ok(None), |row| Ok(Some(row)))
        } else {
            let mut stmt =
                conn.prepare("SELECT name, version FROM workflows WHERE id = ?1 AND is_latest = 1")?;
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
}
