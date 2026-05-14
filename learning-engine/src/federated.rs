use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPattern {
    pub id: String,
    pub workflow_id: String,
    pub node_sequence: Vec<String>,
    pub parameters: HashMap<String, serde_json::Value>,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub sample_count: u32,
    pub cluster_id: String,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedUpdate {
    pub cluster_id: String,
    pub patterns: Vec<WorkflowPattern>,
    pub timestamp: i64,
    pub privacy_epsilon: f32,
    pub aggregated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedPattern {
    pub original_id: String,
    pub node_sequence: Vec<String>,
    pub parameters: HashMap<String, serde_json::Value>,
    pub weighted_success_rate: f64,
    pub aggregated_from: Vec<String>,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedConfig {
    pub cluster_id: String,
    pub coordinator_url: String,
    pub sync_interval_hours: u32,
    pub privacy_epsilon: f32,
    pub min_samples_for_upload: u32,
    pub aggregation_method: AggregationMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum AggregationMethod {
    #[default]
    FedAvg,
    FedWeighted,
    SecureAggregation,
}

impl Default for FederatedConfig {
    fn default() -> Self {
        Self {
            cluster_id: String::new(),
            coordinator_url: String::new(),
            sync_interval_hours: 24,
            privacy_epsilon: 1.0,
            min_samples_for_upload: 10,
            aggregation_method: AggregationMethod::FedAvg,
        }
    }
}

pub struct FederatedClient {
    config: FederatedConfig,
    local_patterns: Arc<RwLock<Vec<WorkflowPattern>>>,
    pending_uploads: Arc<RwLock<Vec<FederatedUpdate>>>,
    last_sync: Arc<RwLock<Option<i64>>>,
}

impl FederatedClient {
    pub fn new(config: FederatedConfig) -> Self {
        Self {
            config,
            local_patterns: Arc::new(RwLock::new(Vec::new())),
            pending_uploads: Arc::new(RwLock::new(Vec::new())),
            last_sync: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn add_pattern(&self, pattern: WorkflowPattern) {
        let mut patterns = self.local_patterns.write().await;
        patterns.push(pattern);
    }

    pub async fn get_local_patterns(&self) -> Vec<WorkflowPattern> {
        self.local_patterns.read().await.clone()
    }

    fn apply_differential_privacy(&self, vector: &[f32]) -> Vec<f32> {
        let sensitivity = 1.0 / self.config.privacy_epsilon;
        
        vector.iter().map(|v| {
            let noise = {
                let scale = sensitivity;
                let uniform: f32 = (rand_simple() - 0.5) * 2.0;
                uniform * scale
            };
            v + noise
        }).collect()
    }

    fn compute_pattern_vector(&self, pattern: &WorkflowPattern) -> Vec<f32> {
        let mut vector = Vec::new();
        
        for node in &pattern.node_sequence {
            let hash = simple_hash(node);
            vector.push((hash % 1000) as f32 / 1000.0);
        }
        
        vector.extend([
            pattern.success_rate as f32,
            (pattern.avg_duration_ms / 10000.0).min(1.0) as f32,
            (pattern.sample_count as f32 / 100.0).min(1.0),
        ]);

        while vector.len() < 128 {
            vector.push(0.0);
        }

        vector.truncate(128);
        self.apply_differential_privacy(&vector)
    }

    pub async fn prepare_update(&self) -> Option<FederatedUpdate> {
        let patterns = self.local_patterns.read().await;
        
        let eligible: Vec<_> = patterns
            .iter()
            .filter(|p| p.sample_count >= self.config.min_samples_for_upload)
            .cloned()
            .collect();

        if eligible.is_empty() {
            return None;
        }

        let mut privacy_vectors = Vec::new();
        let mut processed_patterns = Vec::new();

        for mut pattern in eligible {
            let vector = self.compute_pattern_vector(&pattern);
            pattern.vector = vector.clone();
            privacy_vectors.push(vector);
            processed_patterns.push(pattern);
        }

        Some(FederatedUpdate {
            cluster_id: self.config.cluster_id.clone(),
            patterns: processed_patterns,
            timestamp: chrono::Utc::now().timestamp(),
            privacy_epsilon: self.config.privacy_epsilon,
            aggregated: false,
        })
    }

    pub async fn upload_to_coordinator(&self) -> anyhow::Result<()> {
        let update = self.prepare_update().await;
        
        if let Some(payload) = update {
            let client = reqwest::Client::new();
            let url = format!("{}/federated/upload", self.config.coordinator_url);
            
            let response = client
                .post(&url)
                .json(&payload)
                .send()
                .await?;

            if response.status().is_success() {
                let mut last = self.last_sync.write().await;
                *last = Some(chrono::Utc::now().timestamp());
                
                let mut pending = self.pending_uploads.write().await;
                pending.clear();
            } else {
                let mut pending = self.pending_uploads.write().await;
                pending.push(payload);
            }
        }

        Ok(())
    }

    pub async fn download_global_patterns(&self) -> anyhow::Result<Vec<AggregatedPattern>> {
        let client = reqwest::Client::new();
        let url = format!("{}/federated/patterns", self.config.coordinator_url);
        
        let response = client
            .get(&url)
            .query(&[(&"cluster_id", &self.config.cluster_id)])
            .send()
            .await?;

        if response.status().is_success() {
            let patterns: Vec<AggregatedPattern> = response.json().await?;
            Ok(patterns)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn get_last_sync_time(&self) -> Option<i64> {
        *self.last_sync.read().await
    }

    pub async fn retry_pending_uploads(&self) -> anyhow::Result<u32> {
        let pending = self.pending_uploads.read().await.clone();
        let pending_timestamps: std::collections::HashSet<i64> = pending.iter().map(|u| u.timestamp).collect();
        let mut retried = 0;

        for update in pending {
            let client = reqwest::Client::new();
            let url = format!("{}/federated/upload", self.config.coordinator_url);
            
            if client.post(&url).json(&update).send().await?.status().is_success() {
                retried += 1;
            }
        }

        if retried > 0 {
            let mut p = self.pending_uploads.write().await;
            p.retain(|u| !pending_timestamps.contains(&u.timestamp));
        }

        Ok(retried)
    }
}

pub struct FederatedCoordinator {
    clusters: Arc<RwLock<HashMap<String, FederatedConfig>>>,
    global_patterns: Arc<RwLock<Vec<AggregatedPattern>>>,
    aggregation_method: AggregationMethod,
}

impl FederatedCoordinator {
    pub fn new(method: AggregationMethod) -> Self {
        Self {
            clusters: Arc::new(RwLock::new(HashMap::new())),
            global_patterns: Arc::new(RwLock::new(Vec::new())),
            aggregation_method: method,
        }
    }

    pub async fn register_cluster(&self, config: FederatedConfig) {
        let mut clusters = self.clusters.write().await;
        clusters.insert(config.cluster_id.clone(), config);
    }

    pub async fn receive_update(&self, update: FederatedUpdate) -> anyhow::Result<()> {
        if update.aggregated {
            return Ok(());
        }

        let patterns = self.aggregate_patterns(update.patterns, update.cluster_id).await;
        
        let mut global = self.global_patterns.write().await;
        for pattern in patterns {
            if let Some(existing) = global.iter_mut().find(|p| p.original_id == pattern.original_id) {
                *existing = pattern;
            } else {
                global.push(pattern);
            }
        }

        Ok(())
    }

    async fn aggregate_patterns(&self, patterns: Vec<WorkflowPattern>, cluster_id: String) -> Vec<AggregatedPattern> {
        let mut grouped: HashMap<String, Vec<&WorkflowPattern>> = HashMap::new();
        
        for pattern in &patterns {
            let key = pattern.node_sequence.join(",");
            grouped.entry(key).or_default().push(pattern);
        }

        let mut aggregated = Vec::new();

        for (_, group) in grouped {
            let total_weight: f64 = group.iter().map(|p| p.sample_count as f64).sum();
            
            let node_sequence = group[0].node_sequence.clone();
            let mut parameters = group[0].parameters.clone();
            
            let weighted_success: f64 = group.iter()
                .map(|p| p.success_rate * p.sample_count as f64)
                .sum::<f64>() / total_weight;

            let vectors: Vec<Vec<f32>> = group.iter().map(|p| p.vector.clone()).collect();
            let avg_vector = self.average_vectors(&vectors);

            aggregated.push(AggregatedPattern {
                original_id: group[0].id.clone(),
                node_sequence,
                parameters,
                weighted_success_rate: weighted_success,
                aggregated_from: vec![cluster_id.clone()],
                vector: avg_vector,
            });
        }

        aggregated
    }

    fn average_vectors(&self, vectors: &[Vec<f32>]) -> Vec<f32> {
        if vectors.is_empty() {
            return vec![0.0; 128];
        }

        let len = vectors[0].len();
        let mut result = vec![0.0; len];

        for v in vectors {
            for (i, val) in v.iter().enumerate() {
                result[i] += val;
            }
        }

        let count = vectors.len() as f32;
        for val in &mut result {
            *val /= count;
        }

        result
    }

    pub async fn get_global_patterns(&self) -> Vec<AggregatedPattern> {
        self.global_patterns.read().await.clone()
    }

    pub async fn get_cluster_count(&self) -> usize {
        self.clusters.read().await.len()
    }
}

fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for c in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(c as u64);
    }
    hash
}

fn rand_simple() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos as f32 % 1000.0) / 1000.0
}

pub fn create_federated_client(config: FederatedConfig) -> FederatedClient {
    FederatedClient::new(config)
}

pub fn create_coordinator(method: AggregationMethod) -> FederatedCoordinator {
    FederatedCoordinator::new(method)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_federated_client() {
        let config = FederatedConfig {
            cluster_id: "test-cluster".to_string(),
            coordinator_url: "http://localhost:8080".to_string(),
            ..Default::default()
        };
        
        let client = FederatedClient::new(config);
        let patterns = client.get_local_patterns().await;
        assert!(patterns.is_empty());
    }

    #[tokio::test]
    async fn test_coordinator() {
        let coordinator = FederatedCoordinator::new(AggregationMethod::FedAvg);
        let count = coordinator.get_cluster_count().await;
        assert_eq!(count, 0);
    }
}