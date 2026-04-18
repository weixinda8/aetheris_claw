use crate::protocol::industrial::types::*;
use crate::utils::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

#[async_trait]
pub trait IndustrialProtocol: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn reconnect(&mut self) -> Result<()>;

    fn connection_status(&self) -> ConnectionStatus;
    fn config(&self) -> &IndustrialProtocolConfig;

    async fn read_tag(&self, tag_name: &str) -> Result<DataPoint>;
    async fn read_tags(&self, tag_names: &[String]) -> Result<Vec<DataPoint>>;

    async fn write_tag(&self, request: WriteRequest) -> Result<WriteResult>;
    async fn write_tags(&self, requests: &[WriteRequest]) -> Result<Vec<WriteResult>>;

    async fn subscribe(
        &mut self,
        config: SubscriptionConfig,
    ) -> Result<broadcast::Receiver<DataPoint>>;
    async fn unsubscribe(&mut self) -> Result<()>;

    async fn browse_nodes(&self, root_path: Option<&str>) -> Result<Vec<NodeInfo>>;
}

pub trait IndustrialProtocolFactory: Send + Sync {
    fn create(&self, config: IndustrialProtocolConfig) -> Arc<RwLock<dyn IndustrialProtocol>>;
    fn supported_protocols(&self) -> Vec<IndustrialProtocolType>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub node_name: String,
    pub node_class: NodeClass,
    pub data_type: Option<String>,
    pub description: Option<String>,
    pub children: Vec<NodeInfo>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeClass {
    Object,
    Variable,
    Method,
    ObjectType,
    VariableType,
    ReferenceType,
    DataType,
    View,
}
