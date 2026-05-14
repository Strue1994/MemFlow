use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration, MissedTickBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleMode {
    Interval { seconds: u64 },
    Cron { expression: String },
    EventTriggered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub mode: ScheduleMode,
    pub enabled: bool,
    pub max_concurrent_runs: usize,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            mode: ScheduleMode::Interval { seconds: 300 },
            enabled: true,
            max_concurrent_runs: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStats {
    pub total_scheduled_runs: u64,
    pub successful_runs: u64,
    pub failed_runs: u64,
    pub last_run_timestamp: Option<i64>,
    pub next_run_timestamp: Option<i64>,
}

impl Default for SchedulerStats {
    fn default() -> Self {
        Self {
            total_scheduled_runs: 0,
            successful_runs: 0,
            failed_runs: 0,
            last_run_timestamp: None,
            next_run_timestamp: None,
        }
    }
}

pub struct AutoScheduler {
    config: ScheduleConfig,
    stats: Arc<RwLock<SchedulerStats>>,
    is_running: Arc<RwLock<bool>>,
}

impl AutoScheduler {
    pub fn new(config: ScheduleConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(SchedulerStats::default())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start<F, Fut>(&self, mut callback: F) -> anyhow::Result<()>
    where
        F: Send + Sync + FnMut() -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<()>>,
    {
        let mut running = self.is_running.write().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        if !self.config.enabled {
            return Ok(());
        }

        match &self.config.mode {
            ScheduleMode::Interval { seconds } => {
                self.run_interval_loop(callback).await;
            }
            ScheduleMode::Cron { expression: _ } => {
                self.run_cron_loop(callback).await;
            }
            ScheduleMode::EventTriggered => {
                self.run_event_loop(callback).await;
            }
        }

        Ok(())
    }

    async fn run_interval_loop<F, Fut>(&self, mut callback: F)
    where
        F: Send + Sync + FnMut() -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<()>>,
    {
        if let ScheduleMode::Interval { seconds } = &self.config.mode {
            let mut ticker = interval(Duration::from_secs(*seconds));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                ticker.tick().await;
                
                let stats = self.stats.clone();
                let max_concurrent = self.config.max_concurrent_runs;
                
                {
                    let s = stats.read().await;
                    let active_runs = s.successful_runs + s.failed_runs - s.total_scheduled_runs as u64;
                    if active_runs >= max_concurrent as u64 {
                        continue;
                    }
                }

                let now = chrono::Utc::now().timestamp();
                let mut s = stats.write().await;
                s.total_scheduled_runs += 1;
                s.next_run_timestamp = Some(now + *seconds as i64);

                drop(s);

                let result = callback().await;

                let mut s = stats.write().await;
                s.last_run_timestamp = Some(now);
                match result {
                    Ok(_) => s.successful_runs += 1,
                    Err(_) => s.failed_runs += 1,
                }
            }
        }
    }

    async fn run_cron_loop<F, Fut>(&self, mut callback: F)
    where
        F: Send + Sync + FnMut() -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<()>>,
    {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let _ = callback().await;
        }
    }

    async fn run_event_loop<F, Fut>(&self, mut callback: F)
    where
        F: Send + Sync + FnMut() -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<()>>,
    {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }

    pub async fn stop(&self) {
        let mut running = self.is_running.write().await;
        *running = false;
    }

    pub async fn get_stats(&self) -> SchedulerStats {
        self.stats.read().await.clone()
    }

    pub async fn update_config(&mut self, config: ScheduleConfig) {
        self.config = config;
    }

    pub async fn trigger_now(&self) -> anyhow::Result<()> {
        let mut s = self.stats.write().await;
        s.total_scheduled_runs += 1;
        s.last_run_timestamp = Some(chrono::Utc::now().timestamp());
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerState {
    pub is_running: bool,
    pub config: ScheduleConfig,
    pub stats: SchedulerStats,
}

pub fn create_scheduler(config: ScheduleConfig) -> AutoScheduler {
    AutoScheduler::new(config)
}

pub async fn start_scheduled_learning(
    scheduler: Arc<AutoScheduler>,
    orchestrator: Arc<crate::AutoLearningOrchestrator>,
) -> anyhow::Result<()> {
    let orch = orchestrator.clone();
    scheduler.start(move || {
        let orch = orch.clone();
        async move {
            let _result = orch.run_cycle().await;
            Ok(())
        }
    }).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_creation() {
        let config = ScheduleConfig::default();
        let scheduler = AutoScheduler::new(config);
        let stats = scheduler.get_stats().await;
        assert_eq!(stats.total_scheduled_runs, 0);
    }

    #[tokio::test]
    async fn test_scheduler_stats() {
        let config = ScheduleConfig {
            mode: ScheduleMode::Interval { seconds: 1 },
            enabled: false,
            max_concurrent_runs: 1,
        };
        let scheduler = AutoScheduler::new(config);
        scheduler.trigger_now().await.unwrap();
        let stats = scheduler.get_stats().await;
        assert_eq!(stats.total_scheduled_runs, 1);
    }
}