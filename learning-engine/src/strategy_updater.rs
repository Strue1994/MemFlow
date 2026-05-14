use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use rand::Rng;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub user_embedding: Vec<f32>,
    pub historical_success_rate: f32,
    pub hour_of_day: u32,
    pub day_of_week: u32,
    pub intent: String,
}

impl Context {
    pub fn new() -> Self {
        Self {
            user_embedding: vec![0.0; 128],
            historical_success_rate: 0.5,
            hour_of_day: 12,
            day_of_week: 1,
            intent: String::new(),
        }
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.user_embedding = embedding;
        self
    }

    pub fn with_success_rate(mut self, rate: f32) -> Self {
        self.historical_success_rate = rate;
        self
    }

    pub fn with_time(mut self, hour: u32, day: u32) -> Self {
        self.hour_of_day = hour;
        self.day_of_week = day;
        self
    }

    pub fn with_intent(mut self, intent: &str) -> Self {
        self.intent = intent.to_string();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub workflow_id: String,
    pub description: String,
    pub avg_duration_ms: f32,
    pub success_rate: f32,
}

impl Default for Action {
    fn default() -> Self {
        Self {
            workflow_id: String::new(),
            description: String::new(),
            avg_duration_ms: 0.0,
            success_rate: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub context: Context,
    pub selected_workflow_id: String,
    pub timestamp: i64,
    pub reward: Option<f32>,
}

pub struct StrategyUpdater {
    epsilon: f32,
    q_values: Arc<RwLock<HashMap<String, HashMap<String, f32>>>>,
    decision_history: Arc<RwLock<Vec<Decision>>>,
    learning_rate: f32,
    discount_factor: f32,
}

impl StrategyUpdater {
    pub fn new(epsilon: f32) -> Self {
        Self {
            epsilon,
            q_values: Arc::new(RwLock::new(HashMap::new())),
            decision_history: Arc::new(RwLock::new(Vec::new())),
            learning_rate: 0.1,
            discount_factor: 0.9,
        }
    }

    pub async fn register_actions(&self, context_key: &str, actions: Vec<Action>) {
        let mut q_map = self.q_values.write().await;
        q_map.entry(context_key.to_string())
            .or_insert_with(HashMap::new);
        
        for action in actions {
            if !q_map[context_key].contains_key(&action.workflow_id) {
                q_map.get_mut(context_key).unwrap()
                    .insert(action.workflow_id.clone(), 0.0);
            }
        }
    }

    pub async fn select_action(&self, context: &Context, actions: &[Action]) -> Option<String> {
        if actions.is_empty() {
            return None;
        }

        let context_key = self.get_context_key(context);
        
        if !self.q_values.read().await.contains_key(&context_key) {
            self.register_actions(&context_key, actions.to_vec()).await;
        }

        let mut rng = rand::thread_rng();
        let use_greedy = rng.gen::<f32>() > self.epsilon;

        if use_greedy {
            let q_map = self.q_values.read().await;
            let values = q_map.get(&context_key);
            
            if let Some(action_values) = values {
                let best = action_values.iter()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(k, _)| k.clone());
                return best;
            }
        }

        let idx = (rand::random::<f32>() * actions.len() as f32) as usize;
        Some(actions[idx.min(actions.len() - 1)].workflow_id.clone())
    }

    async fn get_q_value(&self, context_key: &str, action: &str) -> f32 {
        let q_map = self.q_values.read().await;
        q_map.get(context_key)
            .and_then(|m| m.get(action))
            .copied()
            .unwrap_or(0.0)
    }

    pub async fn update(&self, context: &Context, action: &str, reward: f32) {
        let context_key = self.get_context_key(context);
        
        let old_q = self.get_q_value(&context_key, action).await;
        let new_q = old_q + self.learning_rate * (reward - old_q);
        
        {
            let mut q_map = self.q_values.write().await;
            q_map.entry(context_key.clone())
                .or_insert_with(HashMap::new)
                .insert(action.to_string(), new_q);
        }

        let mut history = self.decision_history.write().await;
        let decision = Decision {
            context: context.clone(),
            selected_workflow_id: action.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            reward: Some(reward),
        };
        history.push(decision);
        
        if history.len() > 10000 {
            history.drain(0..1000);
        }
    }

    fn get_context_key(&self, context: &Context) -> String {
        format!("{}_{}_{}", context.intent, context.hour_of_day, context.historical_success_rate > 0.7)
    }

    pub async fn get_policy(&self) -> Vec<(String, HashMap<String, f32>)> {
        let q_map = self.q_values.read().await;
        q_map.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub async fn get_statistics(&self) -> PolicyStats {
        let history = self.decision_history.read().await;
        let total = history.len();
        let rewarded = history.iter().filter(|d| d.reward.unwrap_or(0.0) > 0.0).count();
        
        PolicyStats {
            total_decisions: total,
            successful_decisions: rewarded,
            success_rate: if total > 0 { rewarded as f32 / total as f32 } else { 0.0 },
        }
    }

    pub async fn export_model(&self) -> StrategyModel {
        let q_values = self.q_values.read().await.clone();
        let history_len = self.decision_history.read().await.len();
        
        StrategyModel {
            q_values,
            total_decisions: history_len,
            epsilon: self.epsilon,
            exported_at: chrono::Utc::now().timestamp(),
        }
    }

    pub async fn import_model(&self, model: StrategyModel) {
        let mut q_map = self.q_values.write().await;
        *q_map = model.q_values;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyStats {
    pub total_decisions: usize,
    pub successful_decisions: usize,
    pub success_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyModel {
    pub q_values: HashMap<String, HashMap<String, f32>>,
    pub total_decisions: usize,
    pub epsilon: f32,
    pub exported_at: i64,
}

pub async fn run_online_learning(
    strategy: Arc<StrategyUpdater>,
    context: Context,
    action: String,
    success: bool,
    duration_ms: i64,
) {
    let base_reward = if success { 1.0 } else { -1.0 };
    let bonus = if duration_ms < 1000 { 0.5 } else { 0.0 };
    let total_reward = base_reward + bonus;
    
    strategy.update(&context, &action, total_reward).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_select_action() {
        let strategy = StrategyUpdater::new(0.1);
        let context = Context::new().with_intent("send_email");
        
        let actions = vec![
            Action { workflow_id: "wf1".to_string(), description: "Fast".to_string(), avg_duration_ms: 100.0, success_rate: 0.7 },
            Action { workflow_id: "wf2".to_string(), description: "Reliable".to_string(), avg_duration_ms: 500.0, success_rate: 0.95 },
        ];
        
        let selected = strategy.select_action(&context, &actions).await;
        assert!(selected.is_some());
    }

    #[tokio::test]
    async fn test_update() {
        let strategy = StrategyUpdater::new(0.1);
        let context = Context::new().with_intent("send_email");
        
        strategy.update(&context, "wf1", 1.0).await;
        
        let stats = strategy.get_statistics().await;
        assert_eq!(stats.total_decisions, 1);
    }

    #[tokio::test]
    async fn test_export_import() {
        let strategy = StrategyUpdater::new(0.1);
        let model = strategy.export_model().await;
        assert!(model.epsilon > 0.0);
        
        strategy.import_model(model).await;
        let new_model = strategy.export_model().await;
        assert!(new_model.total_decisions > 0);
    }
}