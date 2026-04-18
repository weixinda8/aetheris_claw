use crate::core::Task;
use crate::utils::{AetherisError, Result};
use tokio::sync::Semaphore;

pub struct TaskExecutor {
    semaphore: Semaphore,
}

impl TaskExecutor {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Semaphore::new(max_concurrent),
        }
    }

    pub async fn execute(&self, task: Task) -> Result<Task> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| AetherisError::Runtime(e.to_string()))?;
        Ok(task)
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new(100)
    }
}
