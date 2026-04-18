pub mod coordination;
pub mod model_deployment;
pub mod sync_strategy;

use crate::digital_twin::{TwinModel, TwinStateUpdate};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeType {
    Cloud,
    Edge,
    Device,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeNode {
    pub id: String,
    pub name: String,
    pub node_type: NodeType,
    pub address: String,
    pub status: NodeStatus,
    pub last_seen: DateTime<Utc>,
    pub capabilities: Vec<String>,
    pub resource_usage: Option<NodeResourceUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeStatus {
    Online,
    Offline,
    Degraded,
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResourceUsage {
    pub memory_mb: f64,
    pub cpu_percent: f64,
    pub disk_percent: f64,
    pub network_in_bytes: u64,
    pub network_out_bytes: u64,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncStrategy {
    RealTime,
    Batched { interval_seconds: u64 },
    OnDemand,
    EventDriven,
}

pub struct GlobalCoordinator {
    nodes: Arc<DashMap<String, EdgeNode>>,
    deployments: Arc<DashMap<String, ModelDeployment>>,
    twin_model: Arc<TwinModel>,
    sync_strategy: Arc<RwLock<SyncStrategy>>,
    pending_updates: Arc<DashMap<String, TwinStateUpdate>>,
}

impl GlobalCoordinator {
    pub fn new(twin_model: Arc<TwinModel>) -> Self {
        Self {
            nodes: Arc::new(DashMap::new()),
            deployments: Arc::new(DashMap::new()),
            twin_model,
            sync_strategy: Arc::new(RwLock::new(SyncStrategy::RealTime)),
            pending_updates: Arc::new(DashMap::new()),
        }
    }

    pub fn register_node(&self, node: EdgeNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn get_node(&self, node_id: &str) -> Option<EdgeNode> {
        self.nodes.get(node_id).map(|n| n.value().clone())
    }

    pub fn list_nodes(&self) -> Vec<EdgeNode> {
        self.nodes.iter().map(|n| n.value().clone()).collect()
    }

    pub fn update_node_status(&self, node_id: &str, status: NodeStatus) -> bool {
        if let Some(mut node) = self.nodes.get_mut(node_id) {
            node.status = status;
            node.last_seen = Utc::now();
            true
        } else {
            false
        }
    }

    pub fn deploy_model(&self, mut deployment: ModelDeployment) -> String {
        deployment.id = uuid::Uuid::new_v4().to_string();
        deployment.status = DeploymentStatus::Pending;
        let id = deployment.id.clone();
        self.deployments.insert(id.clone(), deployment);
        id
    }

    pub fn get_deployment(&self, deployment_id: &str) -> Option<ModelDeployment> {
        self.deployments
            .get(deployment_id)
            .map(|d| d.value().clone())
    }

    pub fn list_deployments(&self, node_id: Option<&str>) -> Vec<ModelDeployment> {
        self.deployments
            .iter()
            .map(|d| d.value().clone())
            .filter(|d| node_id.is_none_or(|id| d.target_node_id == id))
            .collect()
    }

    pub fn update_deployment_status(&self, deployment_id: &str, status: DeploymentStatus) -> bool {
        if let Some(mut deployment) = self.deployments.get_mut(deployment_id) {
            deployment.status = status.clone();
            if status == DeploymentStatus::Deployed {
                deployment.deployed_at = Some(Utc::now());
            }
            true
        } else {
            false
        }
    }

    pub async fn set_sync_strategy(&self, strategy: SyncStrategy) {
        *self.sync_strategy.write().await = strategy;
    }

    pub async fn get_sync_strategy(&self) -> SyncStrategy {
        self.sync_strategy.read().await.clone()
    }

    pub fn queue_update(&self, update: TwinStateUpdate) {
        self.pending_updates.insert(update.entity_id.clone(), update);
    }

    pub fn get_pending_updates(&self) -> Vec<TwinStateUpdate> {
        self.pending_updates
            .iter()
            .map(|u| u.value().clone())
            .collect()
    }

    pub fn clear_synced_updates(&self) {
        self.pending_updates.clear();
    }
}
