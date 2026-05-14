use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

const MAX_RATING_CACHE_SIZE: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rating {
    pub id: String,
    pub workflow_id: String,
    pub user_id: String,
    pub rating: u8,
    pub comment: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub moderated: bool,
}

impl Rating {
    pub fn new(workflow_id: String, user_id: String, rating: u8, comment: Option<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: format!("rating_{}_{}", workflow_id, user_id),
            workflow_id,
            user_id,
            rating: rating.min(5).max(1),
            comment,
            created_at: now,
            updated_at: now,
            moderated: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingStats {
    pub workflow_id: String,
    pub average_rating: f64,
    pub total_ratings: u32,
    pub rating_distribution: HashMap<u8, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRatingState {
    pub ratings: Vec<Rating>,
    pub user_ratings: HashMap<String, Rating>,
}

impl Default for WorkflowRatingState {
    fn default() -> Self {
        Self {
            ratings: Vec::new(),
            user_ratings: HashMap::new(),
        }
    }
}

pub struct RatingService {
    workflow_ratings: Arc<RwLock<HashMap<String, WorkflowRatingState>>>,
    moderate_enabled: bool,
    spam_filter_enabled: bool,
}

impl RatingService {
    pub fn new() -> Self {
        Self {
            workflow_ratings: Arc::new(RwLock::new(HashMap::new())),
            moderate_enabled: false,
            spam_filter_enabled: true,
        }
    }

    pub fn with_moderation(mut self, enabled: bool) -> Self {
        self.moderate_enabled = enabled;
        self
    }

    pub fn with_spam_filter(mut self, enabled: bool) -> Self {
        self.spam_filter_enabled = enabled;
        self
    }

    pub async fn rate(&self, workflow_id: String, user_id: String, rating: u8, comment: Option<String>) -> Result<Rating, String> {
        if rating < 1 || rating > 5 {
            return Err("Rating must be between 1 and 5".to_string());
        }

        let mut state = self.workflow_ratings.write().await;
        let workflow_state = state.entry(workflow_id.clone()).or_insert_with(WorkflowRatingState::default);

        let existing = workflow_state.user_ratings.get(&user_id).cloned();
        
        let new_rating = if let Some(existing) = existing {
            let mut updated = existing;
            updated.rating = rating;
            updated.comment = comment.clone();
            updated.updated_at = chrono::Utc::now().timestamp();
            updated
        } else {
            Rating::new(workflow_id.clone(), user_id.clone(), rating, comment)
        };

        if let Some(idx) = workflow_state.ratings.iter().position(|r| r.user_id == user_id) {
            workflow_state.ratings[idx] = new_rating.clone();
        } else {
            workflow_state.ratings.push(new_rating.clone());
        }

        workflow_state.user_ratings.insert(user_id, new_rating.clone());

        Ok(new_rating)
    }

    pub async fn get_ratings(&self, workflow_id: &str) -> Vec<Rating> {
        let state = self.workflow_ratings.read().await;
        state.get(workflow_id).map(|s| s.ratings.clone()).unwrap_or_default()
    }

    pub async fn get_user_rating(&self, workflow_id: &str, user_id: &str) -> Option<Rating> {
        let state = self.workflow_ratings.read().await;
        state.get(workflow_id).and_then(|s| s.user_ratings.get(user_id).cloned())
    }

    pub async fn get_stats(&self, workflow_id: &str) -> Option<RatingStats> {
        let state = self.workflow_ratings.read().await;
        let workflow_state = state.get(workflow_id)?;

        let ratings = &workflow_state.ratings;
        if ratings.is_empty() {
            return Some(RatingStats {
                workflow_id: workflow_id.to_string(),
                average_rating: 0.0,
                total_ratings: 0,
                rating_distribution: HashMap::new(),
            });
        }

        let sum: u32 = ratings.iter().map(|r| r.rating as u32).sum();
        let avg = sum as f64 / ratings.len() as f64;

        let mut distribution = HashMap::new();
        for r in ratings {
            *distribution.entry(r.rating).or_insert(0) += 1;
        }

        Some(RatingStats {
            workflow_id: workflow_id.to_string(),
            average_rating: (avg * 10.0).round() / 10.0,
            total_ratings: ratings.len() as u32,
            rating_distribution: distribution,
        })
    }

    pub async fn delete_rating(&self, workflow_id: &str, user_id: &str) -> Result<(), String> {
        let mut state = self.workflow_ratings.write().await;
        let workflow_state = state.get_mut(workflow_id).ok_or("Workflow not found")?;

        workflow_state.ratings.retain(|r| r.user_id != user_id);
        workflow_state.user_ratings.remove(user_id);

        Ok(())
    }

    pub async fn search_with_rating(
        &self,
        workflow_ids: &[String],
    ) -> Vec<(String, RatingStats)> {
        let mut results = Vec::new();
        
        for id in workflow_ids {
            if let Some(stats) = self.get_stats(id).await {
                results.push((id.clone(), stats));
            }
        }

        results.sort_by(|(_, a), (_, b)| {
            b.average_rating
                .partial_cmp(&a.average_rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    pub async fn calculate_recommendation_score(&self, workflow_id: &str, downloads: u32) -> f64 {
        let stats = self.get_stats(workflow_id).await;

        let avg_rating = stats.as_ref().map(|s| s.average_rating).unwrap_or(0.0);
        let rating_count = stats.as_ref().map(|s| s.total_ratings).unwrap_or(0) as f64;

        let popularity = if downloads > 0 {
            (downloads as f64).ln().max(0.0)
        } else {
            0.0
        };

        if rating_count == 0.0 && downloads == 0 {
            return 0.0;
        }

        avg_rating * (1.0 + 0.1 * popularity)
    }
}

impl Default for RatingService {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RatingModerator {
    blocked_words: Vec<String>,
    min_comment_length: usize,
    max_comment_length: usize,
}

impl RatingModerator {
    pub fn new() -> Self {
        Self {
            blocked_words: vec![
                "spam".to_string(),
                "bad".to_string(),
            ],
            min_comment_length: 3,
            max_comment_length: 500,
        }
    }

    pub fn with_blocked_words(mut self, words: Vec<String>) -> Self {
        self.blocked_words = words;
        self
    }

    pub fn moderate(&self, comment: &str) -> ModerationResult {
        let len = comment.len();
        
        if len < self.min_comment_length || len > self.max_comment_length {
            return ModerationResult {
                approved: false,
                reason: Some(format!(
                    "Comment length must be between {} and {} characters",
                    self.min_comment_length, self.max_comment_length
                )),
            };
        }

        let lower = comment.to_lowercase();
        for word in &self.blocked_words {
            if lower.contains(&word.to_lowercase()) {
                return ModerationResult {
                    approved: false,
                    reason: Some("Comment contains inappropriate content".to_string()),
                };
            }
        }

        ModerationResult {
            approved: true,
            reason: None,
        }
    }
}

impl Default for RatingModerator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationResult {
    pub approved: bool,
    pub reason: Option<String>,
}

pub fn create_rating_service() -> RatingService {
    RatingService::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rating_creation() {
        let service = RatingService::new();
        let rating = service.rate("wf1".to_string(), "user1".to_string(), 5, Some("Great!".to_string())).await;
        assert_eq!(rating.unwrap().rating, 5);
    }

    #[tokio::test]
    async fn test_rating_update() {
        let service = RatingService::new();
        service.rate("wf1".to_string(), "user1".to_string(), 5, None).await;
        service.rate("wf1".to_string(), "user1".to_string(), 3, None).await;
        
        let ratings = service.get_ratings("wf1").await;
        assert_eq!(ratings.len(), 1);
        assert_eq!(ratings[0].rating, 3);
    }

    #[tokio::test]
    async fn test_stats() {
        let service = RatingService::new();
        service.rate("wf1".to_string(), "user1".to_string(), 5, None).await;
        service.rate("wf1".to_string(), "user2".to_string(), 4, None).await;
        
        let stats = service.get_stats("wf1").await.unwrap();
        assert_eq!(stats.total_ratings, 2);
        assert!((stats.average_rating - 4.5).abs() < 0.1);
    }
}
