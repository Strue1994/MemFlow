-- migrations/001_init.sql
-- Initial MemFlow schema

CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS workflows (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    n8n_json TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS workflow_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    n8n_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (workflow_id) REFERENCES workflows(id)
);

CREATE TABLE IF NOT EXISTS execution_logs (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    version INTEGER,
    status TEXT NOT NULL DEFAULT 'pending',
    started_at TEXT,
    finished_at TEXT,
    duration_ms INTEGER,
    error TEXT,
    result TEXT,
    FOREIGN KEY (workflow_id) REFERENCES workflows(id)
);

CREATE TABLE IF NOT EXISTS skills (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    category TEXT,
    pattern TEXT,
    content TEXT,
    version TEXT NOT NULL DEFAULT '1.0.0',
    execution_count INTEGER DEFAULT 0,
    success_rate REAL DEFAULT 0.0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
