use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuneSample {
    pub id: String,
    pub prompt: String,
    pub original_output: String,
    pub modified_output: Option<String>,
    pub accepted: Option<bool>,
    pub user_id: String,
    pub created_at: i64,
    pub used_for_training: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuneConfig {
    pub model_base: String,
    pub sample_threshold: u32,
    pub qlora_rank: u32,
    pub learning_rate: f64,
    pub epochs: u32,
    pub batch_size: u32,
    pub a_b_test_percentage: f64,
    pub improvement_threshold: f64,
    pub scripts_path: String,
}

impl Default for FineTuneConfig {
    fn default() -> Self {
        Self {
            model_base: "deepseek-coder-6.7b".to_string(),
            sample_threshold: 500,
            qlora_rank: 16,
            learning_rate: 0.0001,
            epochs: 3,
            batch_size: 4,
            a_b_test_percentage: 0.1,
            improvement_threshold: 0.1,
            scripts_path: "./scripts".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuneModel {
    pub id: String,
    pub version: String,
    pub base_model: String,
    pub trained_at: i64,
    pub sample_count: u32,
    pub loss: f64,
    pub status: ModelStatus,
    pub serving: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelStatus {
    Training,
    Testing,
    Ready,
    Failed,
    Retired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestResult {
    pub new_model_id: String,
    pub old_model_id: String,
    pub new_modification_rate: f64,
    pub old_modification_rate: f64,
    pub improvement: f64,
    pub promoted: bool,
    pub tested_at: i64,
}

pub struct FineTuneLoop {
    config: FineTuneConfig,
    samples: Arc<RwLock<VecDeque<FineTuneSample>>>,
    models: Arc<RwLock<Vec<FineTuneModel>>>,
    current_model: Arc<RwLock<Option<String>>>,
    ab_tests: Arc<RwLock<Vec<ABTestResult>>>,
}

impl FineTuneLoop {
    pub fn new(config: FineTuneConfig) -> Self {
        Self {
            config,
            samples: Arc::new(RwLock::new(VecDeque::new())),
            models: Arc::new(RwLock::new(Vec::new())),
            current_model: Arc::new(RwLock::new(None)),
            ab_tests: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn add_sample(&self, sample: FineTuneSample) {
        let mut samples = self.samples.write().await;
        samples.push_back(sample);
        
        if samples.len() > 10000 {
            samples.drain(0..1000);
        }
    }

    pub async fn record_accepted(&self, user_id: &str, prompt: &str, original: &str) {
        let sample = FineTuneSample {
            id: format!("sample_{}", uuid_simple()),
            prompt: prompt.to_string(),
            original_output: original.to_string(),
            modified_output: None,
            accepted: Some(true),
            user_id: user_id.to_string(),
            created_at: chrono::Utc::now().timestamp(),
            used_for_training: false,
        };
        self.add_sample(sample).await;
    }

    pub async fn record_modified(&self, user_id: &str, prompt: &str, original: &str, modified: &str) {
        let sample = FineTuneSample {
            id: format!("sample_{}", uuid_simple()),
            prompt: prompt.to_string(),
            original_output: original.to_string(),
            modified_output: Some(modified.to_string()),
            accepted: Some(false),
            user_id: user_id.to_string(),
            created_at: chrono::Utc::now().timestamp(),
            used_for_training: false,
        };
        self.add_sample(sample).await;
    }

    pub async fn record_rejected(&self, user_id: &str, prompt: &str, output: &str) {
        let sample = FineTuneSample {
            id: format!("sample_{}", uuid_simple()),
            prompt: prompt.to_string(),
            original_output: output.to_string(),
            modified_output: None,
            accepted: Some(false),
            user_id: user_id.to_string(),
            created_at: chrono::Utc::now().timestamp(),
            used_for_training: false,
        };
        self.add_sample(sample).await;
    }

    pub async fn get_sample_count(&self) -> usize {
        self.samples.read().await.len()
    }

    pub async fn should_trigger_training(&self) -> bool {
        self.get_sample_count().await >= self.config.sample_threshold as usize
    }

    pub async fn trigger_training(&self) -> anyhow::Result<FineTuneModel> {
        if !self.should_trigger_training().await {
            return Err(anyhow::anyhow!("Not enough samples"));
        }

        let samples = self.samples.read().await;
        let training_samples: Vec<_> = samples.iter()
            .filter(|s| s.modified_output.is_some() || s.accepted == Some(false))
            .take(self.config.sample_threshold as usize)
            .cloned()
            .collect();
        drop(samples);

        let model_id = format!("model_{}", uuid_simple());
        let version = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();

        let model = FineTuneModel {
            id: model_id.clone(),
            version,
            base_model: self.config.model_base.clone(),
            trained_at: chrono::Utc::now().timestamp(),
            sample_count: training_samples.len() as u32,
            loss: 0.0,
            status: ModelStatus::Training,
            serving: false,
        };

        self.models.write().await.push(model.clone());

        let train_result = self.run_training_script(&model, &training_samples).await;

        let mut models = self.models.write().await;
        if let Some(m) = models.iter_mut().find(|m| m.id == model_id) {
            match train_result {
                Ok(loss) => {
                    m.loss = loss;
                    m.status = ModelStatus::Testing;
                }
                Err(e) => {
                    m.status = ModelStatus::Failed;
                    return Err(e);
                }
            }
        }

        Ok(model)
    }

    async fn run_training_script(&self, model: &FineTuneModel, samples: &[FineTuneSample]) -> anyhow::Result<f64> {
        let script_path = format!("{}/fine_tune.sh", self.config.scripts_path);
        
        let sample_file = format!("/tmp/fine_tune_samples_{}.json", model.id);
        let json = serde_json::to_string(samples).unwrap();
        std::fs::write(&sample_file, json)?;

        let output = std::process::Command::new("bash")
            .arg(&script_path)
            .arg("--model")
            .arg(&model.id)
            .arg("--samples")
            .arg(&sample_file)
            .arg("--rank")
            .arg(self.config.qlora_rank.to_string())
            .arg("--epochs")
            .arg(self.config.epochs.to_string())
            .output()?;

        let _ = std::fs::remove_file(&sample_file);

        if !output.status.success() {
            return Err(anyhow::anyhow!("Training script failed"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let loss: f64 = stdout
            .lines()
            .rev()
            .find(|l| l.contains("loss:"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0.0);

        Ok(loss)
    }

    pub async fn start_ab_test(&self, new_model_id: &str) -> anyhow::Result<()> {
        let models = self.models.read().await;
        let current = self.current_model.read().await;

        let new_model = models.iter().find(|m| m.id == new_model_id);
        if new_model.is_none() {
            return Err(anyhow::anyhow!("Model not found"));
        }

        println!("Starting A/B test for model {} ({}% traffic)", new_model_id, self.config.a_b_test_percentage * 100.0);
        
        Ok(())
    }

    pub async fn record_ab_test_result(&self, new_model_id: &str, old_modification_rate: f64, new_modification_rate: f64) {
        let improvement = (old_modification_rate - new_modification_rate) / old_modification_rate.max(0.001);

        let result = ABTestResult {
            new_model_id: new_model_id.to_string(),
            old_model_id: self.current_model.read().await.clone().unwrap_or_default(),
            new_modification_rate,
            old_modification_rate,
            improvement,
            promoted: improvement > self.config.improvement_threshold,
            tested_at: chrono::Utc::now().timestamp(),
        };

        let mut tests = self.ab_tests.write().await;
        tests.push(result.clone());

        if result.promoted {
            let mut current = self.current_model.write().await;
            *current = Some(new_model_id.to_string());

            let mut models = self.models.write().await;
            for m in models.iter_mut() {
                if m.id == new_model_id {
                    m.status = ModelStatus::Ready;
                    m.serving = true;
                } else if m.status == ModelStatus::Ready {
                    m.status = ModelStatus::Retired;
                    m.serving = false;
                }
            }
        }
    }

    pub async fn get_current_model(&self) -> Option<FineTuneModel> {
        let current = self.current_model.read().await.clone();
        if let Some(id) = current {
            self.models.read().await.iter().find(|m| m.id == id).cloned()
        } else {
            None
        }
    }

    pub async fn get_all_models(&self) -> Vec<FineTuneModel> {
        self.models.read().await.clone()
    }

    pub async fn get_ab_test_history(&self) -> Vec<ABTestResult> {
        self.ab_tests.read().await.clone()
    }

    pub async fn get_stats(&self) -> FineTuneStats {
        let samples = self.samples.read().await;
        
        let accepted = samples.iter().filter(|s| s.accepted == Some(true)).count();
        let modified = samples.iter().filter(|s| s.modified_output.is_some()).count();
        let rejected = samples.iter().filter(|s| s.accepted == Some(false)).count();

        let models = self.models.read().await;
        let serving = models.iter().filter(|m| m.serving).count();
        let training = models.iter().filter(|m| m.status == ModelStatus::Training || m.status == ModelStatus::Testing).count();

        FineTuneStats {
            total_samples: samples.len(),
            accepted_samples: accepted,
            modified_samples: modified,
            rejected_samples: rejected,
            total_models: models.len(),
            serving_models: serving,
            training_models: training,
            next_training_at: if samples.len() < self.config.sample_threshold as usize {
                Some(self.config.sample_threshold as usize - samples.len())
            } else {
                None
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuneStats {
    pub total_samples: usize,
    pub accepted_samples: usize,
    pub modified_samples: usize,
    pub rejected_samples: usize,
    pub total_models: usize,
    pub serving_models: usize,
    pub training_models: usize,
    pub next_training_at: Option<usize>,
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    format!("{:x}", nanos)
}

pub fn create_fine_tune_loop(config: FineTuneConfig) -> FineTuneLoop {
    FineTuneLoop::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fine_tune_creation() {
        let loop_config = FineTuneConfig::default();
        let ft = FineTuneLoop::new(loop_config);
        let count = ft.get_sample_count().await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_sample_recording() {
        let ft = FineTuneLoop::new(FineTuneConfig::default());
        ft.record_accepted("user1", "test prompt", "test output").await;
        let count = ft.get_sample_count().await;
        assert_eq!(count, 1);
    }
}