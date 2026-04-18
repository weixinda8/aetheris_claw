use crate::digital_twin::TwinStateUpdate;
use crate::edge_coordination::{
    DeploymentStatus, EdgeNode, ModelDeployment, NodeResourceUsage, NodeStatus, SyncStrategy,
};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct GlobalCoordinator {
    nodes: Arc<DashMap<String, EdgeNode>>,
    deployments: Arc<DashMap<String, ModelDeployment>>,
    sync_strategy: Arc<RwLock<SyncStrategy>>,
    pending_updates: Arc<DashMap<String, TwinStateUpdate>>,
}

impl GlobalCoordinator {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(DashMap::new()),
            deployments: Arc::new(DashMap::new()),
            sync_strategy: Arc::new(RwLock::new(SyncStrategy::RealTime)),
            pending_updates: Arc::new(DashMap::new()),
        }
    }

    pub fn register_node(&self, mut node: EdgeNode) {
        node.last_seen = Utc::now();
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

    pub fn update_node_resource_usage(&self, node_id: &str, usage: NodeResourceUsage) -> bool {
        if let Some(mut node) = self.nodes.get_mut(node_id) {
            node.resource_usage = Some(usage);
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
        let key = format!(
            "{}-{}",
            update.entity_id,
            update.timestamp.timestamp_nanos_opt().unwrap()
        );
        self.pending_updates.insert(key.clone(), update);
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

    pub fn remove_node(&self, node_id: &str) -> bool {
        self.nodes.remove(node_id).is_some()
    }

    pub fn get_online_nodes(&self) -> Vec<EdgeNode> {
        self.nodes
            .iter()
            .map(|n| n.value().clone())
            .filter(|n| n.status == NodeStatus::Online)
            .collect()
    }
}

impl Default for GlobalCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::digital_twin::{TwinState, TwinStateUpdate};
    use crate::edge_coordination::{
        DeploymentStatus, EdgeNode, ModelDeployment, NodeResourceUsage, NodeStatus, NodeType,
        SyncStrategy,
    };
    use chrono::Utc;
    use std::collections::HashMap;

    #[test]
    fn test_global_coordinator_new() {
        let coordinator = GlobalCoordinator::new();
        assert!(coordinator.list_nodes().is_empty());
        assert!(coordinator.list_deployments(None).is_empty());
    }

    #[test]
    fn test_global_coordinator_default() {
        let coordinator = GlobalCoordinator::default();
        assert!(coordinator.list_nodes().is_empty());
    }

    #[tokio::test]
    async fn test_sync_strategy() {
        let coordinator = GlobalCoordinator::new();
        assert_eq!(
            coordinator.get_sync_strategy().await,
            SyncStrategy::RealTime
        );

        coordinator.set_sync_strategy(SyncStrategy::Batched).await;
        assert_eq!(coordinator.get_sync_strategy().await, SyncStrategy::Batched);
    }

    #[test]
    fn test_register_and_get_node() {
        let coordinator = GlobalCoordinator::new();

        let node = EdgeNode {
            id: "node-1".to_string(),
            name: "Test Node".to_string(),
            node_type: NodeType::Edge,
            status: NodeStatus::Online,
            resource_usage: None,
            last_seen: Utc::now(),
            capabilities: Vec::new(),
            location: None,
        };

        coordinator.register_node(node);

        let retrieved = coordinator.get_node("node-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Node");
    }

    #[test]
    fn test_list_nodes() {
        let coordinator = GlobalCoordinator::new();

        let node1 = EdgeNode {
            id: "node-1".to_string(),
            name: "Node 1".to_string(),
            node_type: NodeType::Edge,
            status: NodeStatus::Online,
            resource_usage: None,
            last_seen: Utc::now(),
            capabilities: Vec::new(),
            location: None,
        };

        let node2 = EdgeNode {
            id: "node-2".to_string(),
            name: "Node 2".to_string(),
            node_type: NodeType::Device,
            status: NodeStatus::Offline,
            resource_usage: None,
            last_seen: Utc::now(),
            capabilities: Vec::new(),
            location: None,
        };

        coordinator.register_node(node1);
        coordinator.register_node(node2);

        let nodes = coordinator.list_nodes();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_update_node_status() {
        let coordinator = GlobalCoordinator::new();

        let node = EdgeNode {
            id: "node-1".to_string(),
            name: "Test Node".to_string(),
            node_type: NodeType::Edge,
            status: NodeStatus::Online,
            resource_usage: None,
            last_seen: Utc::now(),
            capabilities: Vec::new(),
            location: None,
        };

        coordinator.register_node(node);

        let result = coordinator.update_node_status("node-1", NodeStatus::Degraded);
        assert!(result);

        let updated = coordinator.get_node("node-1").unwrap();
        assert_eq!(updated.status, NodeStatus::Degraded);
    }

    #[test]
    fn test_deploy_and_get_model() {
        let coordinator = GlobalCoordinator::new();

        let deployment = ModelDeployment {
            id: String::new(),
            model_id: "model-1".to_string(),
            model_name: "Test Model".to_string(),
            target_node_id: "node-1".to_string(),
            version: "1.0.0".to_string(),
            status: DeploymentStatus::Pending,
            deployed_at: None,
            config: HashMap::new(),
        };

        let deployment_id = coordinator.deploy_model(deployment);
        assert!(!deployment_id.is_empty());

        let retrieved = coordinator.get_deployment(&deployment_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().model_name, "Test Model");
    }

    #[test]
    fn test_update_deployment_status() {
        let coordinator = GlobalCoordinator::new();

        let deployment = ModelDeployment {
            id: String::new(),
            model_id: "model-1".to_string(),
            model_name: "Test Model".to_string(),
            target_node_id: "node-1".to_string(),
            version: "1.0.0".to_string(),
            status: DeploymentStatus::Pending,
            deployed_at: None,
            config: HashMap::new(),
        };

        let deployment_id = coordinator.deploy_model(deployment);

        let result =
            coordinator.update_deployment_status(&deployment_id, DeploymentStatus::Deployed);
        assert!(result);

        let updated = coordinator.get_deployment(&deployment_id).unwrap();
        assert_eq!(updated.status, DeploymentStatus::Deployed);
        assert!(updated.deployed_at.is_some());
    }

    #[test]
    fn test_queue_update() {
        let coordinator = GlobalCoordinator::new();

        let update = TwinStateUpdate {
            entity_id: "entity-1".to_string(),
            state: TwinState::Online,
            properties: None,
            timestamp: Utc::now(),
            source: "test".to_string(),
        };

        coordinator.queue_update(update);

        let pending = coordinator.get_pending_updates();
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn test_clear_synced_updates() {
        let coordinator = GlobalCoordinator::new();

        let update = TwinStateUpdate {
            entity_id: "entity-1".to_string(),
            state: TwinState::Online,
            properties: None,
            timestamp: Utc::now(),
            source: "test".to_string(),
        };

        coordinator.queue_update(update);
        assert_eq!(coordinator.get_pending_updates().len(), 1);

        coordinator.clear_synced_updates();
        assert!(coordinator.get_pending_updates().is_empty());
    }

    #[test]
    fn test_get_online_nodes() {
        let coordinator = GlobalCoordinator::new();

        let node1 = EdgeNode {
            id: "node-1".to_string(),
            name: "Online Node".to_string(),
            node_type: NodeType::Edge,
            status: NodeStatus::Online,
            resource_usage: None,
            last_seen: Utc::now(),
            capabilities: Vec::new(),
            location: None,
        };

        let node2 = EdgeNode {
            id: "node-2".to_string(),
            name: "Offline Node".to_string(),
            node_type: NodeType::Edge,
            status: NodeStatus::Offline,
            resource_usage: None,
            last_seen: Utc::now(),
            capabilities: Vec::new(),
            location: None,
        };

        coordinator.register_node(node1);
        coordinator.register_node(node2);

        let online_nodes = coordinator.get_online_nodes();
        assert_eq!(online_nodes.len(), 1);
        assert_eq!(online_nodes[0].name, "Online Node");
    }

    #[test]
    fn test_remove_node() {
        let coordinator = GlobalCoordinator::new();

        let node = EdgeNode {
            id: "node-1".to_string(),
            name: "Test Node".to_string(),
            node_type: NodeType::Edge,
            status: NodeStatus::Online,
            resource_usage: None,
            last_seen: Utc::now(),
            capabilities: Vec::new(),
            location: None,
        };

        coordinator.register_node(node);
        assert!(coordinator.get_node("node-1").is_some());

        let result = coordinator.remove_node("node-1");
        assert!(result);
        assert!(coordinator.get_node("node-1").is_none());
    }
}
