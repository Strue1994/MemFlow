use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeUnit {
    pub id: String,
    pub content: String,
    pub source: Option<String>,
    pub confidence: f64,
    pub freshness: i64,
    pub is_stale: bool,
    pub tags: Vec<String>,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct KnowledgeFusion {
    similarity_threshold: f64,
}

impl KnowledgeFusion {
    pub fn new(threshold: f64) -> Self {
        Self {
            similarity_threshold: threshold,
        }
    }

    pub fn deduplicate(&self, units: &[KnowledgeUnit]) -> Vec<KnowledgeUnit> {
        let mut unique: Vec<KnowledgeUnit> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        for unit in units {
            let hash = self.compute_hash(&unit.content);
            if !seen.contains(&hash) {
                seen.insert(hash);
                unique.push(unit.clone());
            }
        }

        unique
    }

    fn compute_hash(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    pub fn detect_conflicts(&self, units: &[KnowledgeUnit]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        for i in 0..units.len() {
            for j in (i + 1)..units.len() {
                let similar = self.is_similar(&units[i].content, &units[j].content);
                if similar && units[i].tags == units[j].tags {
                    conflicts.push(Conflict {
                        unit_ids: vec![units[i].id.clone(), units[j].id.clone()],
                        reason: "Similar content with same tags".to_string(),
                    });
                }
            }
        }

        conflicts
    }

    fn is_similar(&self, a: &str, b: &str) -> bool {
        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();

        let a_words: HashSet<_> = a_lower.split_whitespace().collect();
        let b_words: HashSet<_> = b_lower.split_whitespace().collect();

        let intersection: HashSet<_> = a_words.intersection(&b_words).collect();
        let union: HashSet<_> = a_words.union(&b_words).collect();

        if union.is_empty() {
            return false;
        }

        let jaccard = intersection.len() as f64 / union.len() as f64;
        jaccard > self.similarity_threshold
    }

    pub fn assign_confidence(&self, unit: &mut KnowledgeUnit, source: Option<&str>) {
        let base_confidence: f64 = match source {
            Some("github") => 0.9,
            Some("official_doc") => 0.95,
            Some("rss") => 0.7,
            Some(_) => 0.6,
            None => 0.5,
        };

        let freshness_bonus: f64 = if unit.freshness > 86400 * 7 { 0.1 } else { 0.0 };
        let confidence = base_confidence + freshness_bonus;
        unit.confidence = if confidence > 1.0 { 1.0 } else { confidence };
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub unit_ids: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaleDetector {
    pub stale_threshold_days: i64,
}

impl StaleDetector {
    pub fn new(stale_threshold_days: i64) -> Self {
        Self {
            stale_threshold_days,
        }
    }

    pub fn detect(&self, units: &[KnowledgeUnit]) -> Vec<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let threshold = now - (self.stale_threshold_days * 86400);

        units
            .iter()
            .filter(|u| u.updated_at < threshold)
            .map(|u| u.id.clone())
            .collect()
    }

    pub fn is_stale(&self, unit: &KnowledgeUnit) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let threshold = now - (self.stale_threshold_days * 86400);
        unit.updated_at < threshold
    }
}
