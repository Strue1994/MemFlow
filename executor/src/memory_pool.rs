use crate::environment::Environment;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::VecDeque;

pub struct MemoryPool {
    pool: Arc<RwLock<VecDeque<Environment>>>,
    max_size: usize,
}

impl MemoryPool {
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: Arc::new(RwLock::new(VecDeque::new())),
            max_size,
        }
    }

    pub async fn acquire(&self) -> Environment {
        let env = {
            let mut pool = self.pool.write().await;
            pool.pop_front()
        };

        match env {
            Some(e) => e,
            None => Environment::new(),
        }
    }

    pub async fn release(&self, mut env: Environment) {
        let mut pool = self.pool.write().await;
        
        if pool.len() < self.max_size {
            env.clear();
            pool.push_back(env);
        }
    }

    pub async fn clear(&self) {
        let mut pool = self.pool.write().await;
        pool.clear();
    }

    pub async fn stats(&self) -> PoolStats {
        let pool = self.pool.read().await;
        PoolStats {
            max_size: self.max_size,
            available: pool.len(),
        }
    }

    pub async fn warm_up(&self, count: usize) {
        let mut pool = self.pool.write().await;
        for _ in 0..count {
            pool.push_back(Environment::new());
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub max_size: usize,
    pub available: usize,
}

lazy_static::lazy_static! {
    pub static ref ENV_POOL: MemoryPool = {
        let size = std::env::var("ENV_POOL_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);
        MemoryPool::new(size)
    };
}

pub async fn acquire_environment() -> Environment {
    ENV_POOL.acquire().await
}

pub async fn release_environment(env: Environment) {
    ENV_POOL.release(env).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_acquire_release() {
        let pool = MemoryPool::new(5);
        
        let env1 = pool.acquire().await;
        assert!(env1.is_empty());
        
        pool.release(env1).await;
        
        let stats = pool.stats().await;
        assert_eq!(stats.available, 1);
    }

    #[tokio::test]
    async fn test_pool_max_size() {
        let pool = MemoryPool::new(2);
        
        let env1 = pool.acquire().await;
        let env2 = pool.acquire().await;
        let env3 = pool.acquire().await;
        
        assert_eq!(pool.stats().await.available, 0);
        
        pool.release(env1).await;
        pool.release(env2).await;
        pool.release(env3).await;
        
        assert_eq!(pool.stats().await.available, 2);
    }
}