use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MemoryLayer {
    Working,
    ShortTerm,
    LongTerm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayeredMemoryConfig {
    pub working_capacity: usize,
    pub short_term_ttl_days: i64,
    pub short_term_importance_threshold: f32,
    pub long_term_importance_threshold: f32,
    pub decay_rate_base: f32,
}

impl Default for LayeredMemoryConfig {
    fn default() -> Self {
        Self {
            working_capacity: 20,
            short_term_ttl_days: 7,
            short_term_importance_threshold: 0.5,
            long_term_importance_threshold: 0.5,
            decay_rate_base: 0.1,
        }
    }
}

pub struct LayeredMemory {
    working_memory: RwLock<Vec<MemoryEntry>>,
    short_term_memory: RwLock<HashMap<String, MemoryEntry>>,
    long_term_memory: RwLock<HashMap<String, MemoryEntry>>,
    config: LayeredMemoryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub layer: MemoryLayer,
    pub initial_importance: f32,
    pub current_importance: f32,
    pub last_access: i64,
    pub created_at: i64,
    pub access_count: u32,
    pub vector: Vec<f32>,
    pub metadata: serde_json::Value,
}

impl MemoryEntry {
    pub fn new(content: String, importance: f32, metadata: serde_json::Value) -> Self {
        let content_for_embed = content.clone();
        let now = Utc::now().timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            layer: MemoryLayer::Working,
            initial_importance: importance,
            current_importance: importance,
            last_access: now,
            created_at: now,
            access_count: 0,
            vector: Self::simple_embed(&content_for_embed),
            metadata,
        }
    }

    fn simple_embed(text: &str) -> Vec<f32> {
        let mut hash: u64 = 0;
        for (i, c) in text.chars().enumerate() {
            hash = hash.wrapping_add((c as u64).wrapping_mul(i as u64 + 1));
        }
        let mut vec = vec![0.0; 128];
        for i in 0..128 {
            vec[i] = ((hash >> (i % 8)) & 0xFF) as f32 / 255.0;
        }
        vec
    }
}

impl LayeredMemory {
    pub fn new(config: LayeredMemoryConfig) -> Self {
        Self {
            working_memory: RwLock::new(Vec::new()),
            short_term_memory: RwLock::new(HashMap::new()),
            long_term_memory: RwLock::new(HashMap::new()),
            config,
        }
    }

    pub async fn store(&self, entry: MemoryEntry) {
        let mut working = self.working_memory.write().await;
        
        if working.len() >= self.config.working_capacity {
            self.promote_to_short_term(&mut working).await;
        }
        
        working.push(entry);
    }

    async fn promote_to_short_term(&self, working: &mut Vec<MemoryEntry>) {
        if working.is_empty() { return; }
        
        let promoted = working.remove(0);
        let mut short_term = self.short_term_memory.write().await;
        short_term.insert(promoted.id.clone(), promoted);
    }

    pub async fn retrieve(&self, query: &str, k: usize) -> Vec<MemoryEntry> {
        let query_vec = MemoryEntry::simple_embed(query);
        let mut candidates = Vec::new();
        
        {
            let working = self.working_memory.read().await;
            for e in working.iter() {
                candidates.push((e.clone(), self.compute_relevance(&query_vec, e)));
            }
        }
        
        {
            let short_term = self.short_term_memory.read().await;
            for e in short_term.values() {
                candidates.push((e.clone(), self.compute_relevance(&query_vec, e)));
            }
        }
        
        {
            let long_term = self.long_term_memory.read().await;
            for e in long_term.values() {
                candidates.push((e.clone(), self.compute_relevance(&query_vec, e)));
            }
        }
        
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        candidates.into_iter().take(k).map(|(e, _)| e).collect()
    }

    fn compute_relevance(&self, query_vec: &[f32], entry: &MemoryEntry) -> f32 {
        let similarity = dot_product(query_vec, &entry.vector);
        entry.current_importance * (0.5 + similarity * 0.5)
    }

    pub async fn apply_decay(&self, days: i64) -> usize {
        let mut removed = 0;
        
        {
            let mut short_term = self.short_term_memory.write().await;
            let to_remove: Vec<String> = short_term.iter()
                .filter(|(_, e)| e.current_importance < 0.05 && days - (e.last_access / 86400) > 90)
                .map(|(id, _)| id.clone())
                .collect();
            
            for id in &to_remove {
                short_term.remove(id);
            }
            removed += to_remove.len();
        }
        
        {
            let mut long_term = self.long_term_memory.write().await;
            let to_archive: Vec<String> = long_term.iter()
                .filter(|(_, e)| e.current_importance < 0.3 && days - (e.last_access / 86400) > 30)
                .map(|(id, _)| id.clone())
                .collect();
            
            for id in &to_archive {
                long_term.remove(id);
            }
        }
        
        removed
    }

    pub async fn get_stats(&self) -> MemorySystemStats {
        let working = self.working_memory.read().await;
        let short_term = self.short_term_memory.read().await;
        let long_term = self.long_term_memory.read().await;
        
        MemorySystemStats {
            working_count: working.len(),
            short_term_count: short_term.len(),
            long_term_count: long_term.len(),
            total: working.len() + short_term.len() + long_term.len(),
        }
    }
}

fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[derive(Debug, Clone, Serialize)]
pub struct MemorySystemStats {
    pub working_count: usize,
    pub short_term_count: usize,
    pub long_term_count: usize,
    pub total: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_layered_memory() {
        let config = LayeredMemoryConfig::default();
        let memory = LayeredMemory::new(config);
        
        let entry = MemoryEntry::new(
            "Test memory".to_string(),
            0.8,
            serde_json::Value::Null,
        );
        memory.store(entry).await;
        
        let stats = memory.get_stats().await;
        assert_eq!(stats.working_count, 1);
    }
}