use crate::Result;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum RetryStrategy {
    FixedInterval(Duration),
    ExponentialBackoff {
        initial_interval: Duration,
        max_interval: Duration,
        multiplier: f64,
    },
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub strategy: RetryStrategy,
}

impl RetryPolicy {
    pub fn new_fixed(max_attempts: u32, interval: Duration) -> Self {
        Self {
            max_attempts,
            strategy: RetryStrategy::FixedInterval(interval),
        }
    }

    pub fn new_exponential(
        max_attempts: u32,
        initial_interval: Duration,
        max_interval: Duration,
        multiplier: f64,
    ) -> Self {
        Self {
            max_attempts,
            strategy: RetryStrategy::ExponentialBackoff {
                initial_interval,
                max_interval,
                multiplier,
            },
        }
    }

    fn get_delay(&self, attempt: u32) -> Duration {
        match &self.strategy {
            RetryStrategy::FixedInterval(interval) => *interval,
            RetryStrategy::ExponentialBackoff {
                initial_interval,
                max_interval,
                multiplier,
            } => {
                let delay = initial_interval.mul_f64(multiplier.powi(attempt as i32));
                if delay > *max_interval {
                    *max_interval
                } else {
                    delay
                }
            }
        }
    }

    pub async fn execute_with_retry<F, Fut, T>(&self, mut f: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 0..self.max_attempts {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);

                    if attempt < self.max_attempts - 1 {
                        let delay = self.get_delay(attempt);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| crate::AetherisError::Internal("Retry failed".to_string())))
    }
}
