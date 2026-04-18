use crate::protocol::{StatusResponse, TaskRequest, TaskResponse};
use crate::utils::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPMessage {
    pub version: String,
    pub message_type: String,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct MCPClient;

impl MCPClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn send_task(&self, _request: TaskRequest) -> Result<TaskResponse> {
        Err(crate::utils::AetherisError::Protocol(
            "MCP not implemented".to_string(),
        ))
    }

    pub async fn get_status(&self, _task_id: &str) -> Result<StatusResponse> {
        Err(crate::utils::AetherisError::Protocol(
            "MCP not implemented".to_string(),
        ))
    }
}

impl Default for MCPClient {
    fn default() -> Self {
        Self::new()
    }
}
