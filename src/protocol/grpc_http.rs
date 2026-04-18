use crate::protocol::{StatusResponse, TaskRequest, TaskResponse};
use crate::utils::Result;

pub struct HttpApiClient;

impl HttpApiClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn submit_task(&self, _request: TaskRequest) -> Result<TaskResponse> {
        Err(crate::utils::AetherisError::Protocol(
            "HTTP API not implemented".to_string(),
        ))
    }

    pub async fn query_status(&self, _task_id: &str) -> Result<StatusResponse> {
        Err(crate::utils::AetherisError::Protocol(
            "HTTP API not implemented".to_string(),
        ))
    }
}

impl Default for HttpApiClient {
    fn default() -> Self {
        Self::new()
    }
}
