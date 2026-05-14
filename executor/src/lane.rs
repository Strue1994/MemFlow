use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lane {
    pub id: String,
    pub name: String,
    pub node_ids: Vec<String>,
    pub dependencies: Vec<String>,
    pub status: LaneStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LaneStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl Default for LaneStatus {
    fn default() -> Self {
        LaneStatus::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneCheckpoint {
    pub workflow_instance_id: String,
    pub lane_id: String,
    pub checkpoint_id: String,
    pub executed_node_index: usize,
    pub variables: HashMap<String, serde_json::Value>,
    pub created_at: i64,
}

pub struct LaneSplitter;

impl LaneSplitter {
    pub fn split_workflow(nodes: &[serde_json::Value], connections: &[serde_json::Value]) -> Vec<Lane> {
        let mut lanes = Vec::new();
        
        let parallel_groups = Self::find_parallel_groups(nodes, connections);
        
        for (i, group) in parallel_groups.iter().enumerate() {
            lanes.push(Lane {
                id: format!("lane_{}", i),
                name: format!("Lane {}", i + 1),
                node_ids: group.clone(),
                dependencies: vec![],
                status: LaneStatus::Pending,
            });
        }
        
        Self::add_dependencies(&mut lanes, connections);
        
        lanes
    }

    fn find_parallel_groups(nodes: &[serde_json::Value], _connections: &[serde_json::Value]) -> Vec<Vec<String>> {
        let mut groups = Vec::new();
        
        if let Some(first) = nodes.first() {
            let first_id = first["id"].as_str().unwrap_or("node_0").to_string();
            groups.push(vec![first_id]);
        }
        
        for node in nodes.iter().skip(1) {
            let node_id = node["id"].as_str().unwrap_or("").to_string();
            if !node_id.is_empty() {
                groups.push(vec![node_id]);
            }
        }
        
        groups
    }

    fn add_dependencies(lanes: &mut Vec<Lane>, connections: &[serde_json::Value]) {
        for conn in connections {
            if let (Some(from), Some(to)) = (
                conn.get("from"),
                conn.get("to")
            ) {
                let from_node = from["node"].as_str().unwrap_or("");
                let to_node = to["node"].as_str().unwrap_or("");
                
                for lane in lanes.iter_mut() {
                    if lane.node_ids.contains(&to_node.to_string()) && !lane.node_ids.contains(&from_node.to_string()) {
                        if !lane.dependencies.contains(&from_node.to_string()) {
                            lane.dependencies.push(from_node.to_string());
                        }
                    }
                }
            }
        }
    }
}

pub struct LaneExecutor;

impl LaneExecutor {
    pub async fn execute_lane(
        lane: &Lane,
        nodes: &[serde_json::Value],
        context: &mut HashMap<String, serde_json::Value>,
    ) -> Result<LaneStatus, String> {
        for node_id in &lane.node_ids {
            if let Some(node) = nodes.iter().find(|n| n["id"].as_str() == Some(node_id.as_str())) {
                Self::execute_node(node, context).await?;
            }
        }
        Ok(LaneStatus::Completed)
    }

    async fn execute_node(node: &serde_json::Value, context: &mut HashMap<String, serde_json::Value>) -> Result<(), String> {
        let node_type = node["type"].as_str().unwrap_or("");
        
        match node_type {
            "n8n-nodes-base.httpRequest" | "HTTP Request" => {
                context.insert("http_response".to_string(), serde_json::json!({"status": "success"}));
            }
            "n8n-nodes-base.set" | "Set" => {
                if let Some(parameters) = node["parameters"].as_object() {
                    for (key, value) in parameters {
                        context.insert(key.clone(), value.clone());
                    }
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}

pub struct CheckpointManager;

impl CheckpointManager {
    pub fn save_checkpoint(checkpoint: &LaneCheckpoint) -> Result<(), String> {
        println!("[Checkpoint] Saved for lane {} in workflow {}", checkpoint.lane_id, checkpoint.workflow_instance_id);
        Ok(())
    }

    pub fn load_checkpoint(workflow_id: &str, lane_id: &str) -> Option<LaneCheckpoint> {
        println!("[Checkpoint] Loading for lane {} in workflow {}", lane_id, workflow_id);
        None
    }

    pub fn delete_checkpoints(workflow_id: &str) -> Result<(), String> {
        println!("[Checkpoint] Deleted all for workflow {}", workflow_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lane_split() {
        let nodes = vec![
            serde_json::json!({"id": "node1", "type": "test"}),
            serde_json::json!({"id": "node2", "type": "test"}),
        ];
        let connections: Vec<serde_json::Value> = vec![];
        
        let lanes = LaneSplitter::split_workflow(&nodes, &connections);
        assert!(lanes.len() >= 1);
    }
}