use crate::Result;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub duration: Duration,
    pub cancel_on_timeout: bool,
}

impl TimeoutConfig {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            cancel_on_timeout: true,
        }
    }
}

pub struct TimeoutManager;

impl TimeoutManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn with_timeout<Fut, T>(
        &self,
        config: TimeoutConfig,
        future: Fut,
        on_timeout: Option<Box<dyn FnOnce() + Send + 'static>>,
    ) -> Result<T>
    where
        Fut: std::future::Future<Output = Result<T>>,
    {
        tokio::select! {
            result = future => {
                result
            }
            _ = tokio::time::sleep(config.duration) => {
                if let Some(callback) = on_timeout {
                    callback();
                }
                Err(crate::AetherisError::Timeout("Operation timed out".to_string()))
            }
        }
    }
}

impl Default for TimeoutManager {
    fn default() -> Self {
        Self::new()
    }
}
