use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

#[derive(Debug, Clone)]
pub struct ConcurrencyMetrics {
    total_ops: Arc<AtomicU64>,
    successful_ops: Arc<AtomicU64>,
    failed_ops: Arc<AtomicU64>,
    lock_wait_time_ns: Arc<AtomicU64>,
    lock_hold_time_ns: Arc<AtomicU64>,
}

impl ConcurrencyMetrics {
    pub fn new() -> Self {
        Self {
            total_ops: Arc::new(AtomicU64::new(0)),
            successful_ops: Arc::new(AtomicU64::new(0)),
            failed_ops: Arc::new(AtomicU64::new(0)),
            lock_wait_time_ns: Arc::new(AtomicU64::new(0)),
            lock_hold_time_ns: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn record_op(&self, success: bool) {
        self.total_ops.fetch_add(1, Ordering::Relaxed);
        if success {
            self.successful_ops.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_ops.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_lock_wait(&self, duration: Duration) {
        self.lock_wait_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn record_lock_hold(&self, duration: Duration) {
        self.lock_hold_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn get_total_ops(&self) -> u64 {
        self.total_ops.load(Ordering::Relaxed)
    }

    pub fn get_successful_ops(&self) -> u64 {
        self.successful_ops.load(Ordering::Relaxed)
    }

    pub fn get_failed_ops(&self) -> u64 {
        self.failed_ops.load(Ordering::Relaxed)
    }

    pub fn get_success_rate(&self) -> f64 {
        let total = self.get_total_ops();
        if total == 0 {
            return 1.0;
        }
        self.get_successful_ops() as f64 / total as f64
    }

    pub fn get_avg_lock_wait_time(&self) -> Duration {
        let total = self.get_total_ops();
        if total == 0 {
            return Duration::from_nanos(0);
        }
        let total_ns = self.lock_wait_time_ns.load(Ordering::Relaxed);
        Duration::from_nanos(total_ns / total)
    }

    pub fn get_avg_lock_hold_time(&self) -> Duration {
        let total = self.get_total_ops();
        if total == 0 {
            return Duration::from_nanos(0);
        }
        let total_ns = self.lock_hold_time_ns.load(Ordering::Relaxed);
        Duration::from_nanos(total_ns / total)
    }

    pub fn reset(&self) {
        self.total_ops.store(0, Ordering::Relaxed);
        self.successful_ops.store(0, Ordering::Relaxed);
        self.failed_ops.store(0, Ordering::Relaxed);
        self.lock_wait_time_ns.store(0, Ordering::Relaxed);
        self.lock_hold_time_ns.store(0, Ordering::Relaxed);
    }
}

impl Default for ConcurrencyMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ConcurrencyLimiter {
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
    metrics: ConcurrencyMetrics,
}

impl ConcurrencyLimiter {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
            metrics: ConcurrencyMetrics::new(),
        }
    }

    pub async fn acquire(&self) -> Result<ConcurrencyGuard, tokio::sync::AcquireError> {
        let permit = self.semaphore.clone().acquire_owned().await?;
        Ok(ConcurrencyGuard {
            permit,
            metrics: self.metrics.clone(),
            start_time: std::time::Instant::now(),
        })
    }

    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    pub fn metrics(&self) -> &ConcurrencyMetrics {
        &self.metrics
    }
}

#[derive(Debug)]
pub struct ConcurrencyGuard {
    permit: tokio::sync::OwnedSemaphorePermit,
    metrics: ConcurrencyMetrics,
    start_time: std::time::Instant,
}

impl Drop for ConcurrencyGuard {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        self.metrics.record_op(true);
        self.metrics.record_lock_hold(duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_concurrency_metrics() {
        let metrics = ConcurrencyMetrics::new();
        
        metrics.record_op(true);
        metrics.record_op(true);
        metrics.record_op(false);
        
        assert_eq!(metrics.get_total_ops(), 3);
        assert_eq!(metrics.get_successful_ops(), 2);
        assert_eq!(metrics.get_failed_ops(), 1);
        assert!((metrics.get_success_rate() - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_concurrency_limiter() {
        let limiter = ConcurrencyLimiter::new(2);
        
        assert_eq!(limiter.max_concurrent(), 2);
        assert_eq!(limiter.available_permits(), 2);
        
        let _guard1 = limiter.acquire().await.unwrap();
        assert_eq!(limiter.available_permits(), 1);
        
        let _guard2 = limiter.acquire().await.unwrap();
        assert_eq!(limiter.available_permits(), 0);
    }

    #[tokio::test]
    async fn test_concurrent_execution() {
        let limiter = ConcurrencyLimiter::new(2);
        let limiter_clone = limiter.clone();
        
        let handle1 = tokio::spawn(async move {
            let _guard = limiter_clone.acquire().await.unwrap();
            sleep(Duration::from_millis(50)).await;
        });
        
        let handle2 = tokio::spawn(async move {
            let _guard = limiter.acquire().await.unwrap();
            sleep(Duration::from_millis(50)).await;
        });
        
        handle1.await.unwrap();
        handle2.await.unwrap();
        
        assert_eq!(limiter.metrics().get_total_ops(), 2);
    }
}
