use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub entity_type: EntityType,
    pub name: String,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    User,
    Workflow,
    NodeType,
    Session,
    Topic,
    Custom(String),
}

impl Entity {
    pub fn new(id: String, entity_type: EntityType, name: String) -> Self {
        Self {
            id,
            entity_type,
            name,
            properties: HashMap::new(),
        }
    }

    pub fn with_property(mut self, key: String, value: String) -> Self {
        self.properties.insert(key, value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from_id: String,
    pub to_id: String,
    pub relation_type: RelationType,
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    Used,
    Created,
    Preferred,
    Similar,
    Followed,
    Failed,
}

pub struct KnowledgeGraph {
    entities: Arc<RwLock<HashMap<String, Entity>>>,
    relations: Arc<RwLock<Vec<Relation>>>,
    adjacency: Arc<RwLock<HashMap<String, Vec<(String, RelationType)>>>>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            entities: Arc::new(RwLock::new(HashMap::new())),
            relations: Arc::new(RwLock::new(Vec::new())),
            adjacency: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_entity(&self, entity: Entity) {
        let mut entities = self.entities.write().await;
        entities.insert(entity.id.clone(), entity);
    }

    pub async fn add_relation(&self, relation: Relation) {
        let mut relations = self.relations.write().await;
        relations.push(relation.clone());

        let mut adjacency = self.adjacency.write().await;
        adjacency.entry(relation.from_id.clone())
            .or_insert_with(Vec::new)
            .push((relation.to_id, relation.relation_type));
    }

    pub async fn add_entity_with_relation(
        &self,
        entity: Entity,
        relation: Relation,
    ) {
        self.add_entity(entity).await;
        self.add_relation(relation).await;
    }

    pub async fn get_entity(&self, id: &str) -> Option<Entity> {
        let entities = self.entities.read().await;
        entities.get(id).cloned()
    }

    pub async fn get_related_entities(&self, entity_id: &str) -> Vec<(Entity, RelationType)> {
        let adjacency = self.adjacency.read().await;
        let entities = self.entities.read().await;
        
        let related = adjacency.get(entity_id).cloned().unwrap_or_default();
        
        related.into_iter()
            .filter_map(|(target_id, rel_type)| {
                entities.get(&target_id).cloned().map(|e| (e, rel_type))
            })
            .collect()
    }

    pub async fn find_path(&self, from_id: &str, to_id: &str, max_depth: usize) -> Option<Vec<String>> {
        let adjacency = self.adjacency.read().await;
        
        let mut visited = HashSet::new();
        let mut queue = vec![(from_id.to_string(), vec![from_id.to_string()])];
        
        while let Some((current, path)) = queue.pop() {
            if current == to_id {
                return Some(path);
            }
            
            if path.len() > max_depth {
                continue;
            }
            
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            
            if let Some(neighbors) = adjacency.get(&current) {
                for (neighbor, _) in neighbors {
                    if !visited.contains(neighbor) {
                        let mut new_path = path.clone();
                        new_path.push(neighbor.clone());
                        queue.push((neighbor.clone(), new_path));
                    }
                }
            }
        }
        
        None
    }

    pub async fn query_by_type(&self, entity_type: EntityType) -> Vec<Entity> {
        let entities = self.entities.read().await;
        entities.values()
            .filter(|e| e.entity_type == entity_type)
            .cloned()
            .collect()
    }

    pub async fn query_relations(&self, from_id: &str, relation_type: Option<RelationType>) -> Vec<(Entity, RelationType)> {
        let adjacency = self.adjacency.read().await;
        let entities = self.entities.read().await;
        
        let related = adjacency.get(from_id).cloned().unwrap_or_default();
        
        let results: Vec<(Entity, RelationType)> = related.into_iter()
            .filter_map(|(target_id, rel_type)| {
                if let Some(ref filter) = relation_type {
                    if *filter != rel_type {
                        return None;
                    }
                }
                entities.get(&target_id).cloned().map(|e| (e, rel_type))
            })
            .collect();
        
        results
    }

    pub async fn get_statistics(&self) -> GraphStatistics {
        let entities = self.entities.read().await;
        let relations = self.relations.read().await;
        
        let mut type_counts = HashMap::new();
        for entity in entities.values() {
            let type_name = format!("{:?}", entity.entity_type);
            *type_counts.entry(type_name).or_insert(0) += 1;
        }
        
        let mut relation_counts = HashMap::new();
        for rel in relations.iter() {
            let rel_name = format!("{:?}", rel.relation_type);
            *relation_counts.entry(rel_name).or_insert(0) += 1;
        }
        
        GraphStatistics {
            total_entities: entities.len(),
            total_relations: relations.len(),
            entities_by_type: type_counts,
            relations_by_type: relation_counts,
        }
    }

    pub async fn find_similar_entities(&self, entity_id: &str) -> Vec<Entity> {
        let entities = self.entities.read().await;
        
        if let Some(source) = entities.get(entity_id) {
            entities.values()
                .filter(|e| e.id != entity_id && e.entity_type == source.entity_type)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub async fn merge_similar_entities(&self, primary_id: &str, secondary_id: &str) {
        let secondary_properties: std::collections::HashMap<String, String>;
        let secondary_neighbors: Vec<(String, RelationType)>;
        
        {
            let entities = self.entities.read().await;
            if let Some(secondary) = entities.get(secondary_id) {
                secondary_properties = secondary.properties.clone();
            } else {
                return;
            }
        }
        
        {
            let mut adjacency = self.adjacency.write().await;
            if let Some(neighbors) = adjacency.get_mut(secondary_id) {
                secondary_neighbors = neighbors.clone();
                neighbors.clear();
            } else {
                secondary_neighbors = Vec::new();
            }
        }
        
        {
            let mut entities = self.entities.write().await;
            if let Some(primary) = entities.get_mut(primary_id) {
                for (key, value) in &secondary_properties {
                    if !primary.properties.contains_key(key) {
                        primary.properties.insert(key.clone(), value.clone());
                    }
                }
            } else {
                return;
            }
        }
        
        {
            let mut adjacency = self.adjacency.write().await;
            for (target, rel_type) in &secondary_neighbors {
                adjacency.entry(primary_id.to_string())
                    .or_insert_with(Vec::new)
                    .push((target.clone(), rel_type.clone()));
            }
        }
        
        {
            let mut entities = self.entities.write().await;
            entities.remove(secondary_id);
        }
    }
}

fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphStatistics {
    pub total_entities: usize,
    pub total_relations: usize,
    pub entities_by_type: HashMap<String, usize>,
    pub relations_by_type: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_entity_and_relation() {
        let graph = KnowledgeGraph::new();
        
        let user = Entity::new("user1".to_string(), EntityType::User, "Alice".to_string());
        let workflow = Entity::new("wf1".to_string(), EntityType::Workflow, "Data Pipeline".to_string());
        
        graph.add_entity(user.clone()).await;
        graph.add_entity(workflow.clone()).await;
        
        let relation = Relation {
            from_id: "user1".to_string(),
            to_id: "wf1".to_string(),
            relation_type: RelationType::Preferred,
            weight: 0.9,
        };
        graph.add_relation(relation).await;
        
        let related = graph.get_related_entities("user1").await;
        assert_eq!(related.len(), 1);
    }

    #[tokio::test]
    async fn test_path_finding() {
        let graph = KnowledgeGraph::new();
        
        graph.add_entity(Entity::new("a".to_string(), EntityType::User, "User A".to_string())).await;
        graph.add_entity(Entity::new("b".to_string(), EntityType::Topic, "Topic B".to_string())).await;
        graph.add_entity(Entity::new("c".to_string(), EntityType::Workflow, "Workflow C".to_string())).await;
        
        graph.add_relation(Relation { from_id: "a".to_string(), to_id: "b".to_string(), relation_type: RelationType::Used, weight: 1.0 }).await;
        graph.add_relation(Relation { from_id: "b".to_string(), to_id: "c".to_string(), relation_type: RelationType::Similar, weight: 1.0 }).await;
        
        let path = graph.find_path("a", "c", 3).await;
        assert!(path.is_some());
    }
}