pub mod simulator;
pub mod sync;
pub mod twin_model;
pub mod visualization;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TwinEntityType {
    Device,
    Sensor,
    Actuator,
    Process,
    Line,
    Factory,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinEntity {
    pub id: String,
    pub name: String,
    pub entity_type: TwinEntityType,
    pub properties: HashMap<String, serde_json::Value>,
    pub state: TwinState,
    pub parent_id: Option<String>,
    pub children_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TwinState {
    Unknown,
    Offline,
    Online,
    Degraded,
    Failed,
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinStateUpdate {
    pub entity_id: String,
    pub state: TwinState,
    pub properties: Option<HashMap<String, serde_json::Value>>,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinCommand {
    pub id: String,
    pub target_entity_id: String,
    pub command_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub issued_at: DateTime<Utc>,
    pub timeout_seconds: Option<u64>,
    pub status: CommandStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandStatus {
    Pending,
    Executing,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

#[async_trait]
pub trait DigitalTwin: Send + Sync {
    fn get_entity(&self, entity_id: &str) -> Option<TwinEntity>;
    fn list_entities(&self) -> Vec<TwinEntity>;
    async fn update_state(&self, update: TwinStateUpdate) -> crate::utils::Result<()>;
    async fn send_command(&self, command: TwinCommand) -> crate::utils::Result<String>;
    async fn get_command_status(&self, command_id: &str) -> Option<TwinCommand>;
}

pub use simulator::{
    DigitalTwinSimulator, EntityModification, SimulationConfig, SimulationMetrics, SimulationMode,
    SimulationResult, WhatIfScenario,
};
pub use sync::{DigitalTwinSynchronizer, SyncConfig, SyncDirection, SyncStats};
pub use twin_model::TwinModel;
pub use visualization::{TwinVisualizationData, VisualizationConnection, VisualizationEntity};
