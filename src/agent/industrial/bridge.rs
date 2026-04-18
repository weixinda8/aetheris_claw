use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::agent::config::IndustrialProtocolIntegrationConfig;
use crate::protocol::industrial::IndustrialProtocolManager;
use crate::protocol::industrial::traits::IndustrialProtocol;
use crate::protocol::industrial::types::{DataPoint, DataValue, WriteRequest, WriteResult};

#[derive(Debug, Error)]
pub enum ProtocolBridgeError {
    #[error("Protocol not enabled")]
    ProtocolNotEnabled,
    #[error("Protocol config missing")]
    ProtocolConfigMissing,
    #[error("Protocol not connected")]
    ProtocolNotConnected,
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Read error: {0}")]
    ReadError(String),
    #[error("Write error: {0}")]
    WriteError(String),
    #[error("Protocol error: {0}")]
    ProtocolError(String),
}

pub struct IndustrialProtocolBridge {
    config: IndustrialProtocolIntegrationConfig,
    manager: Arc<IndustrialProtocolManager>,
    protocol: Option<Arc<RwLock<dyn IndustrialProtocol + Send + Sync>>>,
    is_connected: Arc<RwLock<bool>>,
}

impl IndustrialProtocolBridge {
    pub fn new(
        config: IndustrialProtocolIntegrationConfig,
        manager: Arc<IndustrialProtocolManager>,
    ) -> Self {
        Self {
            config,
            manager,
            protocol: None,
            is_connected: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub async fn connect(&mut self) -> Result<(), ProtocolBridgeError> {
        if !self.config.enabled {
            return Err(ProtocolBridgeError::ProtocolNotEnabled);
        }

        let protocol_config = self
            .config
            .protocol_config
            .as_ref()
            .ok_or(ProtocolBridgeError::ProtocolConfigMissing)?;

        info!(
            "Connecting to industrial protocol: {:?} at {}:{}",
            protocol_config.protocol_type, protocol_config.endpoint, protocol_config.port
        );

        let protocol = self
            .manager
            .create_protocol(protocol_config.clone())
            .map_err(|e| ProtocolBridgeError::ProtocolError(e.to_string()))?;

        protocol
            .write()
            .await
            .connect()
            .await
            .map_err(|e| ProtocolBridgeError::ConnectionError(e.to_string()))?;

        self.protocol = Some(protocol);
        *self.is_connected.write().await = true;

        info!("Industrial protocol connected successfully");
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), ProtocolBridgeError> {
        if let Some(protocol) = &self.protocol {
            protocol
                .write()
                .await
                .disconnect()
                .await
                .map_err(|e| ProtocolBridgeError::ConnectionError(e.to_string()))?;
            *self.is_connected.write().await = false;
            self.protocol = None;
            info!("Industrial protocol disconnected");
        }
        Ok(())
    }

    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }

    pub async fn read_data_point(&self, tag_name: &str) -> Result<DataPoint, ProtocolBridgeError> {
        if !self.is_connected().await {
            return Err(ProtocolBridgeError::ProtocolNotConnected);
        }

        let mapped_tag = self.map_tag(tag_name).await;

        debug!("Reading data point: {}", mapped_tag);

        let protocol = self
            .protocol
            .as_ref()
            .ok_or(ProtocolBridgeError::ProtocolNotConnected)?;

        let protocol = protocol.read().await;
        let data_point = protocol
            .read_tag(&mapped_tag)
            .await
            .map_err(|e| ProtocolBridgeError::ReadError(e.to_string()))?;

        Ok(data_point)
    }

    pub async fn read_data_points(
        &self,
        tag_names: &[String],
    ) -> Result<Vec<DataPoint>, ProtocolBridgeError> {
        if !self.is_connected().await {
            return Err(ProtocolBridgeError::ProtocolNotConnected);
        }

        let mut mapped_tags = Vec::with_capacity(tag_names.len());
        for tag in tag_names {
            mapped_tags.push(self.map_tag(tag).await);
        }

        debug!("Reading data points: {:?}", mapped_tags);

        let protocol = self
            .protocol
            .as_ref()
            .ok_or(ProtocolBridgeError::ProtocolNotConnected)?;

        let protocol = protocol.read().await;
        protocol
            .read_tags(&mapped_tags)
            .await
            .map_err(|e| ProtocolBridgeError::ReadError(e.to_string()))
    }

    pub async fn write_data_point(
        &self,
        tag_name: &str,
        value: DataValue,
    ) -> Result<WriteResult, ProtocolBridgeError> {
        if !self.is_connected().await {
            return Err(ProtocolBridgeError::ProtocolNotConnected);
        }

        let mapped_tag = self.map_tag(tag_name).await;

        debug!("Writing data point: {} = {:?}", mapped_tag, value);

        let protocol = self
            .protocol
            .as_ref()
            .ok_or(ProtocolBridgeError::ProtocolNotConnected)?;

        let protocol = protocol.read().await;
        let request = WriteRequest {
            tag_name: mapped_tag,
            value,
        };

        let result = protocol
            .write_tag(request)
            .await
            .map_err(|e| ProtocolBridgeError::WriteError(e.to_string()))?;

        Ok(result)
    }

    pub async fn write_data_points(
        &self,
        requests: &[WriteRequest],
    ) -> Result<Vec<WriteResult>, ProtocolBridgeError> {
        if !self.is_connected().await {
            return Err(ProtocolBridgeError::ProtocolNotConnected);
        }

        let mut mapped_requests = Vec::with_capacity(requests.len());
        for req in requests {
            let mapped_tag = self.map_tag(&req.tag_name).await;
            mapped_requests.push(WriteRequest {
                tag_name: mapped_tag,
                value: req.value.clone(),
            });
        }

        debug!("Writing data points: {:?}", mapped_requests);

        let protocol = self
            .protocol
            .as_ref()
            .ok_or(ProtocolBridgeError::ProtocolNotConnected)?;

        let protocol = protocol.read().await;
        protocol
            .write_tags(&mapped_requests)
            .await
            .map_err(|e| ProtocolBridgeError::WriteError(e.to_string()))
    }

    async fn map_tag(&self, tag_name: &str) -> String {
        if let Some(mappings) = &self.config.tag_mappings {
            if let Some(mapped) = mappings.get(tag_name) {
                return mapped.clone();
            }
        }
        tag_name.to_string()
    }

    pub fn update_config(&mut self, config: IndustrialProtocolIntegrationConfig) {
        self.config = config;
        info!("Industrial protocol bridge config updated");
    }
}

impl Drop for IndustrialProtocolBridge {
    fn drop(&mut self) {
        debug!("IndustrialProtocolBridge dropped");
    }
}
