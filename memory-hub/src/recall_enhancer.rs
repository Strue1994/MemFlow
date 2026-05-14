use serde::{Deserialize, Serialize};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallConfig {
    pub boost_increment: f32,
    pub max_boosted_importance: f32,
    pub min_accesses_for_boost: u32,
    pub decay_boost_over_time: bool,
}

impl Default for RecallConfig {
    fn default() -> Self {
        Self {
            boost_increment: 0.1,
            max_boosted_importance: 1.0,
            min_accesses_for_boost: 1,
            decay_boost_over_time: true,
        }
    }
}

pub struct RecallEnhancer {
    config: RecallConfig,
    recent_recalls: Arc<RwLock<Vec<RecallEvent>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallEvent {
    pub memory_id: String,
    pub timestamp: i64,
    pub was_used: bool,
    pub boosted_importance: f32,
}

impl RecallEnhancer {
    pub fn new(config: RecallConfig) -> Self {
        Self {
            config,
            recent_recalls: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn on_retrieve(&self, memory_id: &str, memories: &mut [&mut dyn RecallableMemory]) {
        for memory in memories.iter_mut() {
            if memory.id() == memory_id {
                let old_importance = memory.current_importance();
                let new_importance = (old_importance + self.config.boost_increment)
                    .min(self.config.max_boosted_importance);
                
                memory.set_importance(new_importance);
                memory.set_last_access(Utc::now().timestamp());
                memory.increment_access_count();
                
                let event = RecallEvent {
                    memory_id: memory_id.to_string(),
                    timestamp: Utc::now().timestamp(),
                    was_used: true,
                    boosted_importance: new_importance - old_importance,
                };
                
                let mut recalls = self.recent_recalls.write().await;
                recalls.push(event);
            }
        }
    }

    pub async fn on_use(&self, memory_id: &str, memories: &mut [&mut dyn RecallableMemory]) {
        for memory in memories.iter_mut() {
            if memory.id() == memory_id {
                let additional_boost = self.config.boost_increment * 0.5;
                let old_importance = memory.current_importance();
                let new_importance = (old_importance + additional_boost)
                    .min(self.config.max_boosted_importance);
                
                memory.set_importance(new_importance);
                memory.set_last_access(Utc::now().timestamp());
            }
        }
    }

    pub async fn get_recall_stats(&self) -> RecallStats {
        let recalls = self.recent_recalls.read().await;
        
        let total_recalls = recalls.len();
        let used_recalls = recalls.iter().filter(|r| r.was_used).count();
        let avg_boost = if total_recalls > 0 {
            recalls.iter().map(|r| r.boosted_importance).sum::<f32>() / total_recalls as f32
        } else {
            0.0
        };
        
        RecallStats {
            total_recalls,
            used_recalls,
            unused_recalls: total_recalls - used_recalls,
            average_boost: avg_boost,
        }
    }

    pub async fn decay_boosts(&self, memories: &mut [&mut dyn RecallableMemory], decay_factor: f32) {
        for memory in memories.iter_mut() {
            if memory.access_count() >= self.config.min_accesses_for_boost {
                let current = memory.current_importance();
                let initial = memory.initial_importance();
                
                if current > initial {
                    let excess = current - initial;
                    let decayed_excess = excess * decay_factor;
                    let new_importance = initial + decayed_excess;
                    memory.set_importance(new_importance);
                }
            }
        }
    }

    pub async fn get_boosted_memories(&self) -> Vec<String> {
        let recalls = self.recent_recalls.read().await;
        recalls.iter()
            .filter(|r| r.was_used)
            .map(|r| r.memory_id.clone())
            .collect()
    }
}

pub trait RecallableMemory {
    fn id(&self) -> String;
    fn initial_importance(&self) -> f32;
    fn current_importance(&self) -> f32;
    fn set_importance(&mut self, importance: f32);
    fn last_access(&self) -> i64;
    fn set_last_access(&mut self, timestamp: i64);
    fn access_count(&self) -> u32;
    fn increment_access_count(&mut self);
}

#[derive(Debug, Clone, Serialize)]
pub struct RecallStats {
    pub total_recalls: usize,
    pub used_recalls: usize,
    pub unused_recalls: usize,
    pub average_boost: f32,
}

#[derive(Debug, Serialize)]
pub struct MemoryWithRecall {
    pub id: String,
    pub content: String,
    pub initial_importance: f32,
    pub current_importance: f32,
    pub last_access: i64,
    pub access_count: u32,
    pub vector: Vec<f32>,
}

impl Clone for MemoryWithRecall {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            content: self.content.clone(),
            initial_importance: self.initial_importance,
            current_importance: self.current_importance,
            last_access: self.last_access,
            access_count: self.access_count,
            vector: self.vector.clone(),
        }
    }
}

impl RecallableMemory for MemoryWithRecall {
    fn id(&self) -> String { self.id.clone() }
    fn initial_importance(&self) -> f32 { self.initial_importance }
    fn current_importance(&self) -> f32 { self.current_importance }
    fn set_importance(&mut self, importance: f32) { self.current_importance = importance; }
    fn last_access(&self) -> i64 { self.last_access }
    fn set_last_access(&mut self, timestamp: i64) { self.last_access = timestamp; }
    fn access_count(&self) -> u32 { self.access_count }
    fn increment_access_count(&mut self) { self.access_count += 1; }
}

pub struct PrefetchEngine {
    top_k: usize,
    similarity_threshold: f32,
}

impl PrefetchEngine {
    pub fn new(top_k: usize, similarity_threshold: f32) -> Self {
        Self {
            top_k,
            similarity_threshold,
        }
    }

    pub fn calculate_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        dot_product(a, b)
    }

    pub fn prefetch_for_topic(&self, topic: &str, memories: &[MemoryWithRecall]) -> Vec<MemoryWithRecall> {
        let topic_vec = self.compute_topic_vector(topic);
        
        let mut scored: Vec<(usize, f32)> = memories.iter()
            .enumerate()
            .map(|(i, m)| (i, self.calculate_similarity(&topic_vec, &m.vector)))
            .collect();
        
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        scored.into_iter()
            .filter(|(_, score)| *score >= self.similarity_threshold)
            .take(self.top_k)
            .map(|(i, _)| memories[i].clone())
            .collect()
    }

    fn compute_topic_vector(&self, topic: &str) -> Vec<f32> {
        let mut hash: u64 = 0;
        for (i, c) in topic.chars().enumerate() {
            hash = hash.wrapping_add((c as u64).wrapping_mul(i as u64 + 1));
        }
        let mut vec = vec![0.0; 128];
        for i in 0..128 {
            vec[i] = ((hash >> (i % 8)) & 0xFF) as f32 / 255.0;
        }
        vec
    }

    pub fn compress_similar(&self, memories: &mut Vec<MemoryWithRecall>) -> usize {
        if memories.len() < 2 {
            return 0;
        }
        
        let mut to_remove = Vec::new();
        let mut processed = HashSet::new();
        
        for i in 0..memories.len() {
            if processed.contains(&i) {
                continue;
            }
            
            for j in (i + 1)..memories.len() {
                if processed.contains(&j) {
                    continue;
                }
                
                let similarity = self.calculate_similarity(&memories[i].vector, &memories[j].vector);
                
                if similarity > 0.9 {
                    processed.insert(j);
                    to_remove.push(j);
                    
                    memories[i].content = format!(
                        "{} [merged: {}]",
                        memories[i].content,
                        memories[j].content
                    );
                }
            }
        }
        
        for i in to_remove.iter().rev() {
            memories.remove(*i);
        }
        
        to_remove.len()
    }
}

fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

use std::collections::HashSet;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_recall_enhancer() {
        let config = RecallConfig::default();
        let enhancer = RecallEnhancer::new(config);
        
        let stats = enhancer.get_recall_stats().await;
        assert_eq!(stats.total_recalls, 0);
    }

    #[test]
    fn test_prefetch() {
        let engine = PrefetchEngine::new(5, 0.3);
        
        let memories = vec![
            MemoryWithRecall {
                id: "1".to_string(),
                content: "Python coding".to_string(),
                initial_importance: 0.8,
                current_importance: 0.8,
                last_access: 0,
                access_count: 0,
                vector: vec![0.1; 128],
            },
            MemoryWithRecall {
                id: "2".to_string(),
                content: "JavaScript frontend".to_string(),
                initial_importance: 0.7,
                current_importance: 0.7,
                last_access: 0,
                access_count: 0,
                vector: vec![0.9; 128],
            },
        ];
        
        let prefetched = engine.prefetch_for_topic("python", &memories);
        assert!(!prefetched.is_empty());
    }
}