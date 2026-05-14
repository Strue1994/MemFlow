use std::sync::Arc;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub address: String,
    pub port: u16,
    pub is_leader: bool,
    pub is_healthy: bool,
    pub last_heartbeat: i64,
    pub load: f64,
}

impl NodeInfo {
    pub fn new(id: String, address: String, port: u16) -> Self {
        Self {
            id,
            address,
            port,
            is_leader: false,
            is_healthy: true,
            last_heartbeat: chrono::Utc::now().timestamp(),
            load: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadBalanceStrategy {
    RoundRobin,
    LeastLoaded,
    Random,
}

impl Default for LoadBalanceStrategy {
    fn default() -> Self {
        LoadBalanceStrategy::LeastLoaded
    }
}

struct ClusterState {
    nodes: HashMap<String, NodeInfo>,
    strategy: LoadBalanceStrategy,
    round_robin_index: usize,
    last_election: i64,
}

impl Default for ClusterState {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            strategy: LoadBalanceStrategy::LeastLoaded,
            round_robin_index: 0,
            last_election: 0,
        }
    }
}

static CLUSTER: Lazy<Arc<RwLock<ClusterState>>> = Lazy::new(|| {
    Arc::new(RwLock::new(ClusterState::default()))
});

pub async fn register_node(node: NodeInfo) {
    let mut cluster = CLUSTER.write().await;
    cluster.nodes.insert(node.id.clone(), node);
}

pub async fn unregister_node(node_id: &str) -> bool {
    let mut cluster = CLUSTER.write().await;
    cluster.nodes.remove(node_id).is_some()
}

pub async fn get_nodes() -> Vec<NodeInfo> {
    let cluster = CLUSTER.read().await;
    cluster.nodes.values().cloned().collect()
}

pub async fn get_healthy_nodes() -> Vec<NodeInfo> {
    let cluster = CLUSTER.read().await;
    cluster.nodes.values()
        .filter(|n| n.is_healthy)
        .cloned()
        .collect()
}

pub async fn clear_nodes() {
    let mut cluster = CLUSTER.write().await;
    cluster.nodes.clear();
}

pub async fn get_leader() -> Option<NodeInfo> {
    let cluster = CLUSTER.read().await;
    cluster.nodes.values().find(|n| n.is_leader).cloned()
}

pub async fn update_heartbeat(node_id: &str) -> bool {
    let mut cluster = CLUSTER.write().await;
    if let Some(node) = cluster.nodes.get_mut(node_id) {
        node.last_heartbeat = chrono::Utc::now().timestamp();
        node.is_healthy = true;
        true
    } else {
        false
    }
}

pub async fn update_load(node_id: &str, load: f64) {
    let mut cluster = CLUSTER.write().await;
    if let Some(node) = cluster.nodes.get_mut(node_id) {
        node.load = load;
    }
}

pub async fn set_leader(node_id: &str) {
    let mut cluster = CLUSTER.write().await;
    for (id, node) in cluster.nodes.iter_mut() {
        node.is_leader = id == node_id;
    }
    cluster.last_election = chrono::Utc::now().timestamp();
}

pub async fn cleanup_stale_nodes(timeout_secs: i64) -> Vec<String> {
    let mut cluster = CLUSTER.write().await;
    let now = chrono::Utc::now().timestamp();
    let mut removed = Vec::new();
    
    let stale: Vec<String> = cluster.nodes.iter()
        .filter(|(_, n)| now - n.last_heartbeat > timeout_secs)
        .map(|(id, _)| id.clone())
        .collect();
    
    for id in stale {
        cluster.nodes.remove(&id);
        removed.push(id);
    }
    
    removed
}

pub async fn select_node() -> Option<NodeInfo> {
    let cluster = CLUSTER.read().await;
    let healthy: Vec<&NodeInfo> = cluster.nodes.values()
        .filter(|n| n.is_healthy)
        .collect();
    
    if healthy.is_empty() {
        return None;
    }
    
    match cluster.strategy {
        LoadBalanceStrategy::RoundRobin => {
            let idx = cluster.round_robin_index % healthy.len();
            healthy.get(idx).cloned().cloned()
        }
        LoadBalanceStrategy::LeastLoaded => {
            healthy.iter().min_by(|a, b| a.load.partial_cmp(&b.load).unwrap()).cloned().cloned()
        }
        LoadBalanceStrategy::Random => {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos().hash(&mut hasher);
            let idx = (hasher.finish() as usize) % healthy.len();
            healthy.get(idx).cloned().cloned()
        }
    }
}

pub async fn set_strategy(strategy: LoadBalanceStrategy) {
    let mut cluster = CLUSTER.write().await;
    cluster.strategy = strategy;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatus {
    pub node_count: usize,
    pub healthy_count: usize,
    pub leader: Option<String>,
    pub strategy: LoadBalanceStrategy,
}

pub async fn get_cluster_status() -> ClusterStatus {
    let cluster = CLUSTER.read().await;
    let healthy = cluster.nodes.values().filter(|n| n.is_healthy).count();
    let leader = cluster.nodes.values().find(|n| n.is_leader).map(|n| n.id.clone());
    
    ClusterStatus {
        node_count: cluster.nodes.len(),
        healthy_count: healthy,
        leader,
        strategy: cluster.strategy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cluster_operations() {
        clear_nodes().await;
        let node = NodeInfo::new("node1".to_string(), "192.168.1.1".to_string(), 8080);
        register_node(node).await;
        
        let nodes = get_nodes().await;
        assert!(nodes.len() >= 1);
        
        let healthy = get_healthy_nodes().await;
        assert!(healthy.len() >= 1);
    }

    #[tokio::test]
    async fn test_leader_election() {
        clear_nodes().await;
        let node1 = NodeInfo::new("node1".to_string(), "192.168.1.1".to_string(), 8080);
        let node2 = NodeInfo::new("node2".to_string(), "192.168.1.2".to_string(), 8080);
        
        register_node(node1).await;
        register_node(node2).await;
        
        set_leader("node1").await;
        let leader = get_leader().await;
        assert!(leader.is_some());
        assert_eq!(leader.unwrap().id, "node1");
    }

    #[tokio::test]
    async fn test_load_balancing() {
        let mut node1 = NodeInfo::new("node1".to_string(), "192.168.1.1".to_string(), 8080);
        let mut node2 = NodeInfo::new("node2".to_string(), "192.168.1.2".to_string(), 8080);
        node1.load = 10.0;
        node2.load = 5.0;
        
        register_node(node1).await;
        register_node(node2).await;
        
        set_strategy(LoadBalanceStrategy::LeastLoaded).await;
        
        let selected = select_node().await;
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().id, "node2");
    }
}






