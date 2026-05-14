use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct ConcurrencyLimiter {
    semaphore: Semaphore,
    current_count: Arc<AtomicUsize>,
    max_concurrent: usize,
}

impl ConcurrencyLimiter {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Semaphore::new(max_concurrent),
            current_count: Arc::new(AtomicUsize::new(0)),
            max_concurrent,
        }
    }

    pub async fn acquire(&self) -> Result<ConcurrencyPermit, ConcurrencyError> {
        let permit = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.semaphore.acquire()
        ).await.map_err(|_| ConcurrencyError::Timeout)?;

        permit.map(|_p| {
            self.current_count.fetch_add(1, Ordering::SeqCst);
            ConcurrencyPermit {
                limiter: self.current_count.clone(),
            }
        }).map_err(|_| ConcurrencyError::SemaphoreClosed)
    }

    pub fn current_count(&self) -> usize {
        self.current_count.load(Ordering::SeqCst)
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}

pub struct ConcurrencyPermit {
    limiter: Arc<AtomicUsize>,
}

impl Drop for ConcurrencyPermit {
    fn drop(&mut self) {
        self.limiter.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConcurrencyError {
    #[error("Concurrency limit exceeded, request timed out")]
    Timeout,
    #[error("Semaphore closed")]
    SemaphoreClosed,
}

lazy_static::lazy_static! {
    pub static ref CONCURRENCY_LIMITER: ConcurrencyLimiter = {
        let max = std::env::var("MAX_CONCURRENT_WORKFLOWS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);
        ConcurrencyLimiter::new(max)
    };
}