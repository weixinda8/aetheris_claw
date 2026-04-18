use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDeployment {
    pub id: String,
    pub model_id: String,
    pub model_name: String,
    pub version: String,
    pub target_node_id: String,
    pub status: DeploymentStatus,
    pub deployed_at: Option<DateTime<Utc>>,
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeploymentStatus {
    Pending,
    Deploying,
    Deployed,
    Failed,
    Retired,
}
