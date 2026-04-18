use crate::core::realtime_scheduler::types::*;
use crate::utils::Result;
use async_trait::async_trait;
use std::future::Future;
use std::sync::Arc;

#[async_trait]
pub trait RealTimeTask: Send + Sync {
    fn task_id(&self) -> &str;
    fn config(&self) -> &RealTimeTaskConfig;

    async fn execute(&self) -> Result<()>;
    async fn cancel(&self) -> Result<()>;
}

#[async_trait]
pub trait RealTimeScheduler: Send + Sync {
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn pause(&mut self) -> Result<()>;
    async fn resume(&mut self) -> Result<()>;

    async fn submit_task(&mut self, task: Arc<dyn RealTimeTask>) -> Result<()>;
    async fn cancel_task(&mut self, task_id: &str) -> Result<()>;

    fn status(&self) -> SchedulerStatus;
    fn metrics(&self) -> SchedulerMetrics;
    fn task_stats(&self, task_id: &str) -> Option<TaskExecutionStats>;
    fn list_tasks(&self) -> Vec<RealTimeTaskConfig>;
}

pub struct PeriodicTask<F, Fut>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    config: RealTimeTaskConfig,
    task_fn: F,
}

impl<F, Fut> PeriodicTask<F, Fut>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    pub fn new(config: RealTimeTaskConfig, task_fn: F) -> Self {
        Self { config, task_fn }
    }
}

#[async_trait]
impl<F, Fut> RealTimeTask for PeriodicTask<F, Fut>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    fn task_id(&self) -> &str {
        &self.config.task_id
    }

    fn config(&self) -> &RealTimeTaskConfig {
        &self.config
    }

    async fn execute(&self) -> Result<()> {
        (self.task_fn)().await
    }

    async fn cancel(&self) -> Result<()> {
        Ok(())
    }
}
