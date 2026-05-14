use serde::{Deserialize, Serialize};
use serde_json::Value;
use chrono::Utc;
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;

pub mod layered_memory;
pub mod decay;
pub mod graph_memory;
pub mod recall_enhancer;
pub mod embedding;
pub use embedding::{EmbeddingEngine, EmbeddingEngineType, EmbeddingConfig, create_embedding_engine, cosine_similarity, EMBEDDING_DIM};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryType {
    UserPreference,
    WorkflowPattern,
    ErrorRecovery,
    Conversation,
}

impl MemoryType {
    fn as_str(&self) -> &'static str {
        match self {
            MemoryType::UserPreference => "UserPreference",
            MemoryType::WorkflowPattern => "WorkflowPattern",
            MemoryType::ErrorRecovery => "ErrorRecovery",
            MemoryType::Conversation => "Conversation",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "UserPreference" => MemoryType::UserPreference,
            "WorkflowPattern" => MemoryType::WorkflowPattern,
            "ErrorRecovery" => MemoryType::ErrorRecovery,
            _ => MemoryType::Conversation,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub importance: f32,
    pub last_access: i64,
    pub created_at: i64,
    pub ttl: Option<i64>,
    pub metadata: Value,
    pub vector: Vec<f32>,
}

impl MemoryEntry {
    pub fn new(content: String, memory_type: MemoryType, importance: f32, metadata: Value) -> Self {
        let now = Utc::now().timestamp();
        let vector = embedding::LocalEmbedding::compute_simhash(&content);
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            memory_type: memory_type.clone(),
            importance,
            last_access: now,
            created_at: now,
            ttl: None,
            metadata,
            vector,
        }
    }

    fn simple_embed(text: &str) -> Vec<f32> {
        // DEPRECATED: use embedding::LocalEmbedding::compute_simhash instead
        // Kept for backward compatibility
        crate::embedding::LocalEmbedding::compute_simhash(text)
    }
}

struct SqliteDb {
    conn: rusqlite::Connection,
}

impl SqliteDb {
    fn new(path: &PathBuf) -> Result<Self, rusqlite::Error> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                memory_type TEXT NOT NULL,
                importance REAL NOT NULL,
                last_access INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                ttl INTEGER,
                metadata TEXT NOT NULL,
                vector BLOB NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_type ON memories(memory_type)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_last_access ON memories(last_access)",
            [],
        )?;
        Ok(Self { conn })
    }

    fn insert(&self, entry: &MemoryEntry) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO memories (id, content, memory_type, importance, last_access, created_at, ttl, metadata, vector) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                entry.id,
                entry.content,
                entry.memory_type.as_str(),
                entry.importance,
                entry.last_access,
                entry.created_at,
                entry.ttl,
                serde_json::to_string(&entry.metadata).unwrap_or_default(),
                bincode::serialize(&entry.vector).unwrap_or_default(),
            ],
        )?;
        Ok(())
    }

    fn get_all(&self) -> Result<Vec<MemoryEntry>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, memory_type, importance, last_access, created_at, ttl, metadata, vector FROM memories"
        )?;
        let entries = stmt.query_map([], |row| {
            let vector_bytes: Vec<u8> = row.get(8)?;
            let vector: Vec<f32> = bincode::deserialize(&vector_bytes).unwrap_or_default();
            let metadata_str: String = row.get(7)?;
            let metadata: Value = serde_json::from_str(&metadata_str).unwrap_or(Value::Null);
            Ok(MemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                memory_type: MemoryType::from_str(&row.get::<_, String>(2)?),
                importance: row.get(3)?,
                last_access: row.get(4)?,
                created_at: row.get(5)?,
                ttl: row.get(6)?,
                metadata,
                vector,
            })
        })?;
        entries.collect()
    }

    fn delete(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM memories WHERE id = ?1", [id])?;
        Ok(())
    }

    fn count(&self) -> Result<usize, rusqlite::Error> {
        let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    fn update_last_access(&self, id: &str) -> Result<(), rusqlite::Error> {
        let now = Utc::now().timestamp();
        self.conn.execute(
            "UPDATE memories SET last_access = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )?;
        Ok(())
    }
}

pub struct MemoryStore {
    cache: Arc<RwLock<HashMap<String, MemoryEntry>>>,
    db: Arc<std::sync::Mutex<Option<SqliteDb>>>,
    max_entries: usize,
    compression_threshold: f32,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            db: Arc::new(std::sync::Mutex::new(None)),
            max_entries: 10000,
            compression_threshold: 0.3,
        }
    }

    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    pub fn with_compression_threshold(mut self, threshold: f32) -> Self {
        self.compression_threshold = threshold;
        self
    }

    pub fn with_persistence(&self, db_path: PathBuf) -> Result<(), anyhow::Error> {
        let db = SqliteDb::new(&db_path)?;
        let entries = db.get_all()?;
        let cache = self.cache.clone();
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let mut cache = cache.write().await;
                for entry in entries {
                    cache.insert(entry.id.clone(), entry);
                }
            });
        });
        handle.join().map_err(|e| anyhow::anyhow!("Thread failed: {:?}", e))?;
        let mut db_lock = self.db.lock().unwrap();
        *db_lock = Some(db);
        Ok(())
    }

    pub async fn store(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        {
            let mut cache = self.cache.write().await;
            cache.insert(entry.id.clone(), entry.clone());
        }
        if let Ok(db_lock) = self.db.lock() {
            if let Some(db) = db_lock.as_ref() {
                db.insert(&entry)?;
            }
        }
        Ok(())
    }

    pub async fn retrieve(&self, query: &str, k: usize) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        let query_vec = MemoryEntry::simple_embed(query);
        let entries = self.cache.read().await;
        
        let mut scored: Vec<(String, f32)> = entries.values()
            .map(|e| {
                let score = dot_product(&query_vec, &e.vector);
                (e.id.clone(), score)
            })
            .collect();
        
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        let mut results = Vec::new();
        for (id, _) in scored.iter().take(k) {
            if let Some(entry) = entries.get(id) {
                let mut entry = entry.clone();
                entry.last_access = Utc::now().timestamp();
                results.push(entry.clone());
                if let Ok(db_lock) = self.db.lock() {
                    if let Some(db) = db_lock.as_ref() {
                        let _ = db.update_last_access(id);
                    }
                }
            }
        }
        Ok(results)
    }

    pub async fn get_all(&self) -> Vec<MemoryEntry> {
        let entries = self.cache.read().await;
        entries.values().cloned().collect()
    }

    pub async fn delete(&self, id: &str) -> Result<(), anyhow::Error> {
        {
            let mut entries = self.cache.write().await;
            entries.remove(id);
        }
        if let Ok(db_lock) = self.db.lock() {
            if let Some(db) = db_lock.as_ref() {
                db.delete(id)?;
            }
        }
        Ok(())
    }

    pub async fn count(&self) -> usize {
        let entries = self.cache.read().await;
        entries.len()
    }

    pub async fn compress(&self) -> Result<usize, anyhow::Error> {
        let count = self.count().await;
        if count <= self.max_entries {
            return Ok(0);
        }

        let mut to_remove = Vec::new();
        
        {
            let cache = self.cache.read().await;
            let mut entries: Vec<&MemoryEntry> = cache.values().collect();
            entries.sort_by(|a, b| {
                let score_a = a.importance * (1.0 - (Utc::now().timestamp() - a.last_access) as f32 / (30.0 * 24.0 * 3600.0));
                let score_b = b.importance * (1.0 - (Utc::now().timestamp() - b.last_access) as f32 / (30.0 * 24.0 * 3600.0));
                score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
            });

            let to_keep = self.max_entries / 2;
            for entry in entries.iter().skip(to_keep) {
                to_remove.push(entry.id.clone());
            }
        }

        {
            let mut cache = self.cache.write().await;
            for id in &to_remove {
                cache.remove(id);
            }
        }

        if let Ok(db_lock) = self.db.lock() {
            if let Some(db) = db_lock.as_ref() {
                for id in &to_remove {
                    let _ = db.delete(id);
                }
            }
        }

        Ok(to_remove.len())
    }

    pub async fn cleanup_expired(&self) -> Result<usize, anyhow::Error> {
        let now = Utc::now().timestamp();
        let mut to_remove = Vec::new();

        {
            let entries = self.cache.read().await;
            for (id, entry) in entries.iter() {
                if let Some(ttl) = entry.ttl {
                    if now > entry.created_at + ttl {
                        to_remove.push(id.clone());
                    }
                }
            }
        }

        {
            let mut cache = self.cache.write().await;
            for id in &to_remove {
                cache.remove(id);
            }
        }

        if let Ok(db_lock) = self.db.lock() {
            if let Some(db) = db_lock.as_ref() {
                for id in &to_remove {
                    let _ = db.delete(id);
                }
            }
        }

        Ok(to_remove.len())
    }

    pub async fn get_stats(&self) -> MemoryStats {
        let entries = self.cache.read().await;
        let count = entries.len();
        let mut by_type: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        
        for entry in entries.values() {
            *by_type.entry(entry.memory_type.as_str().to_string()).or_insert(0) += 1;
        }

        MemoryStats {
            total_entries: count,
            max_entries: self.max_entries,
            by_type,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryStats {
    pub total_entries: usize,
    pub max_entries: usize,
    pub by_type: std::collections::HashMap<String, usize>,
}

fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

pub async fn start_server(port: &str, db_path: Option<String>) -> anyhow::Result<()> {
    use axum::{Router, routing::{delete, get, post}, extract::{Path, Query, State}, response::IntoResponse, http::StatusCode};
    use std::sync::Arc;

    #[derive(Clone)]
    struct AppState {
        store: Arc<MemoryStore>,
    }

    #[derive(Deserialize)]
    struct StoreRequest {
        content: String,
        #[serde(rename = "type")]
        memory_type: String,
        importance: f32,
        metadata: Option<Value>,
    }

    #[derive(Deserialize)]
    struct SearchQuery {
        q: Option<String>,
        k: Option<usize>,
    }

    async fn store_handler(
        State(state): State<AppState>,
        axum::Json(req): axum::Json<StoreRequest>,
    ) -> impl IntoResponse {
        let mem_type = match req.memory_type.as_str() {
            "UserPreference" => MemoryType::UserPreference,
            "WorkflowPattern" => MemoryType::WorkflowPattern,
            "ErrorRecovery" => MemoryType::ErrorRecovery,
            _ => MemoryType::Conversation,
        };
        let entry = MemoryEntry::new(req.content, mem_type, req.importance, req.metadata.unwrap_or(Value::Null));
        if let Err(e) = state.store.store(entry).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
        (StatusCode::OK, "ok".to_string())
    }

    async fn search_handler(
        State(state): State<AppState>,
        Query(query): Query<SearchQuery>,
    ) -> impl IntoResponse {
        let q = query.q.unwrap_or_default();
        let k = query.k.unwrap_or(5);
        match state.store.retrieve(&q, k).await {
            Ok(results) => (StatusCode::OK, serde_json::to_string(&results).unwrap_or_default()),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    }

    async fn list_handler(State(state): State<AppState>) -> impl IntoResponse {
        let entries = state.store.get_all().await;
        (StatusCode::OK, serde_json::to_string(&entries).unwrap_or_default())
    }

    async fn delete_handler(
        Path(id): Path<String>,
        State(state): State<AppState>,
    ) -> impl IntoResponse {
        if let Err(e) = state.store.delete(&id).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
        (StatusCode::OK, "ok".to_string())
    }

    async fn stats_handler(State(state): State<AppState>) -> impl IntoResponse {
        let count = state.store.count().await;
        (StatusCode::OK, serde_json::json!({ "count": count }).to_string())
    }

    let store = Arc::new(MemoryStore::new());
    if let Some(path) = db_path {
        if let Err(e) = store.with_persistence(PathBuf::from(&path)) {
            eprintln!("Warning: Failed to initialize SQLite persistence: {}", e);
        } else {
            println!("Memory hub SQLite persistence initialized at {}", path);
        }
    }
    let state = AppState { store };
    let app = Router::new()
        .route("/memories", post(store_handler))
        .route("/memories", get(list_handler))
        .route("/memories/search", get(search_handler))
        .route("/memories/:id", delete(delete_handler))
        .route("/stats", get(stats_handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Memory hub listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

pub fn run_main(port: &str, db_path: Option<String>) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if let Err(e) = start_server(port, db_path).await {
            eprintln!("Memory hub error: {}", e);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_memory_store() {
        let store = MemoryStore::new();
        let entry = MemoryEntry::new(
            "User prefers dark mode".to_string(),
            MemoryType::UserPreference,
            0.8,
            Value::Null,
        );
        store.store(entry).await.unwrap();
        let results = store.retrieve("dark mode", 1).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let store = MemoryStore::new();
        store.with_persistence(db_path).unwrap();
        
        let entry = MemoryEntry::new(
            "Persistent memory".to_string(),
            MemoryType::WorkflowPattern,
            0.9,
            Value::Null,
        );
        store.store(entry).await.unwrap();
        
        let store2 = MemoryStore::new();
        let db_path2 = temp_dir.path().join("test.db");
        store2.with_persistence(db_path2).unwrap();
        
        let count = store2.count().await;
        assert_eq!(count, 1);
    }
}

pub use layered_memory::{LayeredMemory, LayeredMemoryConfig, MemoryLayer, MemoryEntry as LayeredMemoryEntry, MemorySystemStats};
pub use decay::{EbbinghausDecay, DecayConfig, DecayableMemory, DecayEntry, UserPreferenceDecay};
pub use graph_memory::{KnowledgeGraph, Entity, EntityType, Relation, RelationType, GraphStatistics};
pub use recall_enhancer::{RecallEnhancer, RecallConfig, RecallEvent, RecallStats, RecallableMemory, MemoryWithRecall, PrefetchEngine};

