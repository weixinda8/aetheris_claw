use crate::protocol::industrial::traits::*;
use crate::protocol::industrial::types::*;
use crate::protocol::industrial::ConnectionManager;
use crate::utils::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, broadcast};

#[cfg(feature = "opcua")]
mod real {
    use super::*;
    use opcua::client::{Client, Session};
    use opcua::core::supported_message::SupportedMessage;
    use opcua::types::{
        AttributeId, DataValue as OpcUaDataValue, NodeId, QualifiedName, ReferenceDescription,
        ReferenceTypeId, StatusCode, Variant,
    };

    pub struct OpcUaProtocol {
        config: IndustrialProtocolConfig,
        connection_manager: Arc<ConnectionManager>,
        subscription_tx: Option<broadcast::Sender<DataPoint>>,
        inner_state: Arc<Mutex<Option<OpcUaInnerState>>>,
    }

    struct OpcUaInnerState {
        client: Option<Client>,
        session: Option<Session>,
        subscription_id: Option<u32>,
        monitored_items: Vec<String>,
        reconnect_attempts: u32,
    }

    impl OpcUaProtocol {
        pub fn new(config: IndustrialProtocolConfig) -> Self {
            let connection_manager = Arc::new(ConnectionManager::new(config.clone()));
            Self {
                config,
                connection_manager,
                subscription_tx: None,
                inner_state: Arc::new(Mutex::new(None)),
            }
        }

        async fn create_inner_state(&self) -> OpcUaInnerState {
            OpcUaInnerState {
                client: None,
                session: None,
                subscription_id: None,
                monitored_items: Vec::new(),
                reconnect_attempts: 0,
            }
        }

        fn build_endpoint_url(&self) -> String {
            let security_config = self.config.security_config.as_ref();
            let scheme = if security_config.map(|s| s.use_tls).unwrap_or(false) {
                "opc.tcp"
            } else {
                "opc.tcp"
            };
            format!("{}://{}:{}", scheme, self.config.endpoint, self.config.port)
        }

        fn convert_opcua_data_value(&self, node_id: &str, value: OpcUaDataValue) -> DataPoint {
            let data_value = if let Some(variant) = value.value {
                match variant {
                    Variant::Boolean(v) => DataValue::Boolean(v),
                    Variant::Int8(v) => DataValue::Int8(v),
                    Variant::Int16(v) => DataValue::Int16(v),
                    Variant::Int32(v) => DataValue::Int32(v),
                    Variant::Int64(v) => DataValue::Int64(v),
                    Variant::UInt8(v) => DataValue::UInt8(v),
                    Variant::UInt16(v) => DataValue::UInt16(v),
                    Variant::UInt32(v) => DataValue::UInt32(v),
                    Variant::UInt64(v) => DataValue::UInt64(v),
                    Variant::Float(v) => DataValue::Float32(v),
                    Variant::Double(v) => DataValue::Float64(v),
                    Variant::String(v) => DataValue::String(v.as_ref().to_string()),
                    Variant::ByteString(v) => DataValue::ByteArray(v.as_ref().to_vec()),
                    _ => DataValue::Float64(0.0),
                }
            } else {
                DataValue::Float64(0.0)
            };

            let quality = match value.status {
                StatusCode::Good => DataQuality::Good,
                StatusCode::Uncertain => DataQuality::Uncertain,
                _ => DataQuality::Bad,
            };

            DataPoint {
                tag_name: node_id.to_string(),
                timestamp: value
                    .source_timestamp
                    .map(|t| t.into())
                    .unwrap_or_else(|| chrono::Utc::now()),
                value: data_value,
                quality,
            }
        }

        fn convert_to_opcua_variant(&self, value: &DataValue) -> Variant {
            match value {
                DataValue::Boolean(v) => Variant::Boolean(*v),
                DataValue::Int8(v) => Variant::Int8(*v),
                DataValue::Int16(v) => Variant::Int16(*v),
                DataValue::Int32(v) => Variant::Int32(*v),
                DataValue::Int64(v) => Variant::Int64(*v),
                DataValue::UInt8(v) => Variant::UInt8(*v),
                DataValue::UInt16(v) => Variant::UInt16(*v),
                DataValue::UInt32(v) => Variant::UInt32(*v),
                DataValue::UInt64(v) => Variant::UInt64(*v),
                DataValue::Float32(v) => Variant::Float(*v),
                DataValue::Float64(v) => Variant::Double(*v),
                DataValue::String(v) => Variant::String(v.into()),
                DataValue::ByteArray(v) => Variant::ByteString(v.as_slice().into()),
            }
        }

        async fn browse_node_recursive(
            &self,
            session: &Session,
            node_id: NodeId,
            node_name: String,
            max_depth: usize,
        ) -> Result<NodeInfo> {
            let node_class = self.get_node_class(session, &node_id).await?;
            let data_type = self
                .get_node_data_type(session, &node_id, &node_class)
                .await?;

            let mut children = Vec::new();
            if max_depth > 0 {
                let browse_result = session
                    .browse((
                        &node_id,
                        0u32,
                        ReferenceTypeId::HierarchicalReferences,
                        true,
                        0xFF,
                    ))
                    .await?;
                if let Some(references) = browse_result.references {
                    for reference in references {
                        if let Ok(child_node_id) = reference.node_id.node_id() {
                            let child_name = reference.display_name.text.as_ref().to_string();
                            let child_info = self
                                .browse_node_recursive(
                                    session,
                                    child_node_id,
                                    child_name,
                                    max_depth - 1,
                                )
                                .await?;
                            children.push(child_info);
                        }
                    }
                }
            }

            Ok(NodeInfo {
                node_id: node_id.to_string(),
                node_name,
                node_class,
                data_type,
                description: None,
                children,
            })
        }

        async fn get_node_class(&self, session: &Session, node_id: &NodeId) -> Result<NodeClass> {
            let value = session
                .read(node_id.clone(), AttributeId::NodeClass)
                .await?;
            if let Some(Variant::Int32(class)) = value.value {
                match class {
                    1 => Ok(NodeClass::Object),
                    2 => Ok(NodeClass::Variable),
                    4 => Ok(NodeClass::Method),
                    8 => Ok(NodeClass::ObjectType),
                    16 => Ok(NodeClass::VariableType),
                    32 => Ok(NodeClass::ReferenceType),
                    64 => Ok(NodeClass::DataType),
                    128 => Ok(NodeClass::View),
                    _ => Ok(NodeClass::Object),
                }
            } else {
                Ok(NodeClass::Object)
            }
        }

        async fn get_node_data_type(
            &self,
            session: &Session,
            node_id: &NodeId,
            node_class: &NodeClass,
        ) -> Result<Option<String>> {
            if *node_class != NodeClass::Variable {
                return Ok(None);
            }

            let value = session.read(node_id.clone(), AttributeId::DataType).await?;
            if let Some(Variant::NodeId(data_type_id)) = value.value {
                Ok(Some(data_type_id.to_string()))
            } else {
                Ok(None)
            }
        }
    }

    #[async_trait]
    impl IndustrialProtocol for OpcUaProtocol {
        async fn connect(&mut self) -> Result<()> {
            self.connection_manager
                .set_status(ConnectionStatus::Connecting)
                .await;

            let mut state = self.inner_state.lock().await;
            *state = Some(self.create_inner_state().await);
            let inner = state.as_mut().unwrap();

            let endpoint_url = self.build_endpoint_url();
            log::info!("OPC UA connecting to {}", endpoint_url);

            let mut client = Client::new();
            client.set_session_timeout(self.config.timeout_ms);

            let session = client.connect_to_endpoint(&endpoint_url).await?;

            inner.client = Some(client);
            inner.session = Some(session);
            inner.reconnect_attempts = 0;

            self.connection_manager
                .set_status(ConnectionStatus::Connected)
                .await;
            self.connection_manager.update_heartbeat();

            log::info!("OPC UA connected successfully");
            Ok(())
        }

        async fn disconnect(&mut self) -> Result<()> {
            self.connection_manager
                .set_status(ConnectionStatus::Disconnected)
                .await;

            let mut state = self.inner_state.lock().await;
            if let Some(inner) = state.as_mut() {
                if let Some(session) = inner.session.take() {
                    session.disconnect().await?;
                }
                inner.client = None;
                inner.subscription_id = None;
                inner.monitored_items.clear();
            }
            *state = None;

            self.subscription_tx = None;
            log::info!("OPC UA disconnected");
            Ok(())
        }

        async fn reconnect(&mut self) -> Result<()> {
            self.connection_manager
                .set_status(ConnectionStatus::Reconnecting)
                .await;

            let state = self.inner_state.lock().await;
            if let Some(inner) = state.as_ref() {
                if inner.reconnect_attempts >= self.config.max_reconnect_attempts {
                    drop(state);
                    self.connection_manager
                        .set_status(ConnectionStatus::Error)
                        .await;
                    return Err(crate::utils::AetherisError::Protocol(
                        "Max reconnect attempts exceeded".to_string(),
                    ));
                }
            }
            drop(state);

            log::info!("OPC UA attempting to reconnect");
            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.config.reconnect_interval_ms,
            ))
            .await;

            self.disconnect().await?;
            self.connect().await?;

            Ok(())
        }

        fn connection_status(&self) -> ConnectionStatus {
            self.connection_manager.get_status()
        }

        fn config(&self) -> &IndustrialProtocolConfig {
            &self.config
        }

        async fn read_tag(&self, tag_name: &str) -> Result<DataPoint> {
            let state = self.inner_state.lock().await;
            let inner = state.as_ref().ok_or_else(|| {
                crate::utils::AetherisError::Protocol("Not connected".to_string())
            })?;
            let session = inner
                .session
                .as_ref()
                .ok_or_else(|| crate::utils::AetherisError::Protocol("No session".to_string()))?;

            let node_id = NodeId::from_str(tag_name).map_err(|e| {
                crate::utils::AetherisError::Protocol(format!("Invalid node ID: {}", e))
            })?;

            let value = session.read(node_id, AttributeId::Value).await?;
            Ok(self.convert_opcua_data_value(tag_name, value))
        }

        async fn read_tags(&self, tag_names: &[String]) -> Result<Vec<DataPoint>> {
            let state = self.inner_state.lock().await;
            let inner = state.as_ref().ok_or_else(|| {
                crate::utils::AetherisError::Protocol("Not connected".to_string())
            })?;
            let session = inner
                .session
                .as_ref()
                .ok_or_else(|| crate::utils::AetherisError::Protocol("No session".to_string()))?;

            let mut results = Vec::with_capacity(tag_names.len());
            for tag_name in tag_names {
                match self.read_tag(tag_name).await {
                    Ok(dp) => results.push(dp),
                    Err(e) => {
                        log::error!("Failed to read tag {}: {}", tag_name, e);
                        results.push(DataPoint {
                            tag_name: tag_name.clone(),
                            timestamp: chrono::Utc::now(),
                            value: DataValue::Float64(0.0),
                            quality: DataQuality::BadNotConnected,
                        });
                    }
                }
            }
            Ok(results)
        }

        async fn write_tag(&self, request: WriteRequest) -> Result<WriteResult> {
            let state = self.inner_state.lock().await;
            let inner = state.as_ref().ok_or_else(|| {
                crate::utils::AetherisError::Protocol("Not connected".to_string())
            })?;
            let session = inner
                .session
                .as_ref()
                .ok_or_else(|| crate::utils::AetherisError::Protocol("No session".to_string()))?;

            let node_id = NodeId::from_str(&request.tag_name).map_err(|e| {
                crate::utils::AetherisError::Protocol(format!("Invalid node ID: {}", e))
            })?;
            let variant = self.convert_to_opcua_variant(&request.value);

            let status = session
                .write((node_id, AttributeId::Value, variant))
                .await?;

            Ok(WriteResult {
                tag_name: request.tag_name,
                success: status.is_good(),
                error_message: if status.is_good() {
                    None
                } else {
                    Some(format!("Write failed with status: {:?}", status))
                },
            })
        }

        async fn write_tags(&self, requests: &[WriteRequest]) -> Result<Vec<WriteResult>> {
            let mut results = Vec::with_capacity(requests.len());
            for request in requests {
                results.push(self.write_tag(request.clone()).await?);
            }
            Ok(results)
        }

        async fn subscribe(
            &mut self,
            config: SubscriptionConfig,
        ) -> Result<broadcast::Receiver<DataPoint>> {
            let (tx, rx) = broadcast::channel(config.queue_size);
            self.subscription_tx = Some(tx.clone());

            let state = self.inner_state.lock().await;
            let inner = state.as_ref().ok_or_else(|| {
                crate::utils::AetherisError::Protocol("Not connected".to_string())
            })?;
            let session = inner
                .session
                .as_ref()
                .ok_or_else(|| crate::utils::AetherisError::Protocol("No session".to_string()))?;

            let tx_clone = tx.clone();
            let config_clone = config.clone();
            let protocol_clone = self.clone();

            tokio::spawn(async move {
                loop {
                    let state = protocol_clone.inner_state.lock().await;
                    if let Some(inner) = state.as_ref() {
                        if let Some(session) = inner.session.as_ref() {
                            for tag_name in &config_clone.tag_names {
                                if let Ok(dp) = protocol_clone.read_tag(tag_name).await {
                                    let _ = tx_clone.send(dp);
                                }
                            }
                        }
                    }
                    drop(state);
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        config_clone.sampling_interval_ms as u64,
                    ))
                    .await;
                }
            });

            log::info!("OPC UA subscribed to {} tags", config.tag_names.len());
            Ok(rx)
        }

        async fn unsubscribe(&mut self) -> Result<()> {
            self.subscription_tx = None;
            log::info!("OPC UA unsubscribed");
            Ok(())
        }

        async fn browse_nodes(&self, root_path: Option<&str>) -> Result<Vec<NodeInfo>> {
            let state = self.inner_state.lock().await;
            let inner = state.as_ref().ok_or_else(|| {
                crate::utils::AetherisError::Protocol("Not connected".to_string())
            })?;
            let session = inner
                .session
                .as_ref()
                .ok_or_else(|| crate::utils::AetherisError::Protocol("No session".to_string()))?;

            let root_node_id = if let Some(path) = root_path {
                NodeId::from_str(path).map_err(|e| {
                    crate::utils::AetherisError::Protocol(format!("Invalid root node: {}", e))
                })?
            } else {
                NodeId::root_folder_id()
            };

            let root_node = self
                .browse_node_recursive(session, root_node_id, "Root".to_string(), 3)
                .await?;

            Ok(vec![root_node])
        }
    }

    impl Clone for OpcUaProtocol {
        fn clone(&self) -> Self {
            Self {
                config: self.config.clone(),
                connection_manager: self.connection_manager.clone(),
                subscription_tx: self.subscription_tx.clone(),
                inner_state: self.inner_state.clone(),
            }
        }
    }

    pub struct OpcUaProtocolFactory;

    impl IndustrialProtocolFactory for OpcUaProtocolFactory {
        fn create(&self, config: IndustrialProtocolConfig) -> Arc<RwLock<dyn IndustrialProtocol>> {
            Arc::new(RwLock::new(OpcUaProtocol::new(config)))
        }

        fn supported_protocols(&self) -> Vec<IndustrialProtocolType> {
            vec![IndustrialProtocolType::OpcUa]
        }
    }
}

#[cfg(not(feature = "opcua"))]
mod mock {
    use super::*;

    pub struct OpcUaProtocol {
        config: IndustrialProtocolConfig,
        connection_manager: Arc<ConnectionManager>,
        subscription_tx: Option<broadcast::Sender<DataPoint>>,
        inner_state: Arc<Mutex<Option<OpcUaInnerState>>>,
    }

    struct OpcUaInnerState {
        session: Option<()>,
        subscription_id: Option<u32>,
        monitored_items: Vec<String>,
    }

    impl OpcUaProtocol {
        pub fn new(config: IndustrialProtocolConfig) -> Self {
            let connection_manager = Arc::new(ConnectionManager::new(config.clone()));
            Self {
                config,
                connection_manager,
                subscription_tx: None,
                inner_state: Arc::new(Mutex::new(None)),
            }
        }

        async fn create_inner_state(&self) -> OpcUaInnerState {
            OpcUaInnerState {
                session: None,
                subscription_id: None,
                monitored_items: Vec::new(),
            }
        }
    }

    #[async_trait]
    impl IndustrialProtocol for OpcUaProtocol {
        async fn connect(&mut self) -> Result<()> {
            self.connection_manager
                .set_status(ConnectionStatus::Connecting)
                .await;

            let mut state = self.inner_state.lock().await;
            *state = Some(self.create_inner_state().await);

            log::info!(
                "OPC UA (mock) connecting to {}:{}",
                self.config.endpoint,
                self.config.port
            );

            self.connection_manager
                .set_status(ConnectionStatus::Connected)
                .await;
            self.connection_manager.update_heartbeat();

            Ok(())
        }

        async fn disconnect(&mut self) -> Result<()> {
            self.connection_manager
                .set_status(ConnectionStatus::Disconnected)
                .await;

            let mut state = self.inner_state.lock().await;
            *state = None;

            log::info!("OPC UA (mock) disconnected");
            Ok(())
        }

        async fn reconnect(&mut self) -> Result<()> {
            self.disconnect().await?;
            self.connect().await
        }

        fn connection_status(&self) -> ConnectionStatus {
            if self.connection_manager.is_connected() {
                ConnectionStatus::Connected
            } else {
                ConnectionStatus::Disconnected
            }
        }

        fn config(&self) -> &IndustrialProtocolConfig {
            &self.config
        }

        async fn read_tag(&self, tag_name: &str) -> Result<DataPoint> {
            Ok(DataPoint {
                tag_name: tag_name.to_string(),
                timestamp: chrono::Utc::now(),
                value: DataValue::Float64(0.0),
                quality: DataQuality::Good,
            })
        }

        async fn read_tags(&self, tag_names: &[String]) -> Result<Vec<DataPoint>> {
            let mut results = Vec::with_capacity(tag_names.len());
            for tag_name in tag_names {
                results.push(self.read_tag(tag_name).await?);
            }
            Ok(results)
        }

        async fn write_tag(&self, request: WriteRequest) -> Result<WriteResult> {
            Ok(WriteResult {
                tag_name: request.tag_name,
                success: true,
                error_message: None,
            })
        }

        async fn write_tags(&self, requests: &[WriteRequest]) -> Result<Vec<WriteResult>> {
            let mut results = Vec::with_capacity(requests.len());
            for request in requests {
                results.push(self.write_tag(request.clone()).await?);
            }
            Ok(results)
        }

        async fn subscribe(
            &mut self,
            config: SubscriptionConfig,
        ) -> Result<broadcast::Receiver<DataPoint>> {
            let (tx, rx) = broadcast::channel(config.queue_size);
            self.subscription_tx = Some(tx.clone());

            log::info!(
                "OPC UA (mock) subscribed to {} tags",
                config.tag_names.len()
            );

            Ok(rx)
        }

        async fn unsubscribe(&mut self) -> Result<()> {
            self.subscription_tx = None;
            log::info!("OPC UA (mock) unsubscribed");
            Ok(())
        }

        async fn browse_nodes(&self, _root_path: Option<&str>) -> Result<Vec<NodeInfo>> {
            Ok(vec![NodeInfo {
                node_id: "root".to_string(),
                node_name: "Root".to_string(),
                node_class: NodeClass::Object,
                data_type: None,
                description: None,
                children: Vec::new(),
            }])
        }
    }

    impl Clone for OpcUaProtocol {
        fn clone(&self) -> Self {
            Self {
                config: self.config.clone(),
                connection_manager: self.connection_manager.clone(),
                subscription_tx: self.subscription_tx.clone(),
                inner_state: self.inner_state.clone(),
            }
        }
    }

    pub struct OpcUaProtocolFactory;

    impl IndustrialProtocolFactory for OpcUaProtocolFactory {
        fn create(&self, config: IndustrialProtocolConfig) -> Arc<RwLock<dyn IndustrialProtocol>> {
            Arc::new(RwLock::new(OpcUaProtocol::new(config)))
        }

        fn supported_protocols(&self) -> Vec<IndustrialProtocolType> {
            vec![IndustrialProtocolType::OpcUa]
        }
    }
}

#[cfg(feature = "opcua")]
pub use real::*;

#[cfg(not(feature = "opcua"))]
pub use mock::*;
