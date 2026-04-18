use crate::protocol::industrial::connection::ConnectionManager;
use crate::protocol::industrial::traits::*;
use crate::protocol::industrial::types::*;
use crate::utils::{AetherisError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, broadcast};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ModbusRegisterType {
    Coil,
    DiscreteInput,
    HoldingRegister,
    InputRegister,
}

#[derive(Debug, Clone)]
struct ModbusTagAddress {
    register_type: ModbusRegisterType,
    address: u16,
    bit_offset: Option<u8>,
    data_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusTagConfig {
    pub tag_name: String,
    pub register_type: String,
    pub address: u16,
    pub bit_offset: Option<u8>,
    pub data_type: String,
}

enum ModbusClientState {
    Disconnected,
}

struct ModbusInnerState {
    slave_id: u8,
    tag_address_map: HashMap<String, ModbusTagAddress>,
    monitored_registers: Vec<u16>,
}

impl ModbusInnerState {
    fn new(slave_id: u8, tag_configs: Vec<ModbusTagConfig>) -> Self {
        let mut tag_address_map = HashMap::new();
        for config in tag_configs {
            let register_type = match config.register_type.to_lowercase().as_str() {
                "coil" | "fc01" => ModbusRegisterType::Coil,
                "discrete_input" | "fc02" => ModbusRegisterType::DiscreteInput,
                "holding_register" | "fc03" => ModbusRegisterType::HoldingRegister,
                "input_register" | "fc04" => ModbusRegisterType::InputRegister,
                _ => ModbusRegisterType::HoldingRegister,
            };

            tag_address_map.insert(
                config.tag_name,
                ModbusTagAddress {
                    register_type,
                    address: config.address,
                    bit_offset: config.bit_offset,
                    data_type: config.data_type,
                },
            );
        }

        ModbusInnerState {
            slave_id,
            tag_address_map,
            monitored_registers: Vec::new(),
        }
    }
}

pub struct ModbusProtocol {
    config: IndustrialProtocolConfig,
    connection_manager: Arc<ConnectionManager>,
    subscription_tx: Option<broadcast::Sender<DataPoint>>,
    inner_state: Arc<Mutex<Option<ModbusInnerState>>>,
    use_simulation_mode: bool,
    simulation_data: Arc<RwLock<HashMap<String, DataValue>>>,
}

impl ModbusProtocol {
    pub fn new(config: IndustrialProtocolConfig) -> Self {
        let connection_manager = Arc::new(ConnectionManager::new(config.clone()));

        let _tag_configs = config
            .extra_config
            .get("tags")
            .and_then(|v| serde_json::from_value::<Vec<ModbusTagConfig>>(v.clone()).ok())
            .unwrap_or_default();

        let simulation_data = Arc::new(RwLock::new(HashMap::new()));

        Self {
            config,
            connection_manager,
            subscription_tx: None,
            inner_state: Arc::new(Mutex::new(None)),
            use_simulation_mode: false,
            simulation_data,
        }
    }

    async fn create_inner_state(&self) -> ModbusInnerState {
        let slave_id = self
            .config
            .extra_config
            .get("slave_id")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u8;

        let tag_configs = self
            .config
            .extra_config
            .get("tags")
            .and_then(|v| serde_json::from_value::<Vec<ModbusTagConfig>>(v.clone()).ok())
            .unwrap_or_default();

        ModbusInnerState::new(slave_id, tag_configs)
    }

    async fn connect_real(&mut self) -> Result<()> {
        log::info!("Modbus TCP using simulation mode");
        Ok(())
    }

    async fn disconnect_real(&mut self) -> Result<()> {
        log::info!("Modbus TCP disconnected");
        Ok(())
    }

    fn parse_register_value(&self, value: u16, data_type: &str) -> Result<DataValue> {
        match data_type.to_lowercase().as_str() {
            "uint16" | "u16" => Ok(DataValue::UInt16(value)),
            "int16" | "i16" => Ok(DataValue::Int16(value as i16)),
            "uint8" | "u8" => Ok(DataValue::UInt8((value & 0xFF) as u8)),
            "int8" | "i8" => Ok(DataValue::Int8((value & 0xFF) as i8)),
            "bool" | "boolean" => Ok(DataValue::Boolean(value != 0)),
            _ => Ok(DataValue::UInt16(value)),
        }
    }

    fn data_value_to_u16(&self, value: &DataValue) -> Result<u16> {
        match value {
            DataValue::UInt16(v) => Ok(*v),
            DataValue::Int16(v) => Ok(*v as u16),
            DataValue::UInt8(v) => Ok(*v as u16),
            DataValue::Int8(v) => Ok(*v as u16),
            DataValue::Boolean(v) => Ok(if *v { 1 } else { 0 }),
            _ => Err(AetherisError::Protocol(
                "Unsupported data type for register".to_string(),
            )),
        }
    }

    async fn read_tag_simulation(&self, tag_name: &str) -> Result<DataPoint> {
        let sim_data = self.simulation_data.read().await;
        let value = sim_data
            .get(tag_name)
            .cloned()
            .unwrap_or(DataValue::UInt16(0));

        Ok(DataPoint {
            tag_name: tag_name.to_string(),
            timestamp: chrono::Utc::now(),
            value,
            quality: DataQuality::Good,
        })
    }

    async fn write_tag_simulation(&self, request: WriteRequest) -> Result<WriteResult> {
        let mut sim_data = self.simulation_data.write().await;
        sim_data.insert(request.tag_name.clone(), request.value);

        Ok(WriteResult {
            tag_name: request.tag_name,
            success: true,
            error_message: None,
        })
    }
}

#[async_trait]
impl IndustrialProtocol for ModbusProtocol {
    async fn connect(&mut self) -> Result<()> {
        self.connection_manager
            .set_status(ConnectionStatus::Connecting)
            .await;

        let mut state = self.inner_state.lock().await;
        *state = Some(self.create_inner_state().await);
        drop(state);

        match self.connect_real().await {
            Ok(_) => {
                self.use_simulation_mode = false;
                self.connection_manager
                    .set_status(ConnectionStatus::Connected)
                    .await;
                self.connection_manager.update_heartbeat();
                Ok(())
            }
            Err(e) => {
                log::warn!(
                    "Failed to connect to real Modbus device, falling back to simulation mode: {}",
                    e
                );
                self.use_simulation_mode = true;
                self.connection_manager
                    .set_status(ConnectionStatus::Connected)
                    .await;
                Ok(())
            }
        }
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connection_manager
            .set_status(ConnectionStatus::Disconnected)
            .await;

        let mut state = self.inner_state.lock().await;
        *state = None;

        log::info!("Modbus disconnected");
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
        self.read_tag_simulation(tag_name).await
    }

    async fn read_tags(&self, tag_names: &[String]) -> Result<Vec<DataPoint>> {
        let mut results = Vec::with_capacity(tag_names.len());
        for tag_name in tag_names {
            match self.read_tag(tag_name).await {
                Ok(dp) => results.push(dp),
                Err(e) => {
                    log::error!("Failed to read tag {}: {}", tag_name, e);
                    results.push(DataPoint {
                        tag_name: tag_name.clone(),
                        timestamp: chrono::Utc::now(),
                        value: DataValue::UInt16(0),
                        quality: DataQuality::Bad,
                    });
                }
            }
        }
        Ok(results)
    }

    async fn write_tag(&self, request: WriteRequest) -> Result<WriteResult> {
        self.write_tag_simulation(request).await
    }

    async fn write_tags(&self, requests: &[WriteRequest]) -> Result<Vec<WriteResult>> {
        let mut results = Vec::with_capacity(requests.len());
        for request in requests {
            match self.write_tag(request.clone()).await {
                Ok(res) => results.push(res),
                Err(e) => {
                    log::error!("Failed to write tag {}: {}", request.tag_name, e);
                    results.push(WriteResult {
                        tag_name: request.tag_name.clone(),
                        success: false,
                        error_message: Some(e.to_string()),
                    });
                }
            }
        }
        Ok(results)
    }

    async fn subscribe(
        &mut self,
        config: SubscriptionConfig,
    ) -> Result<broadcast::Receiver<DataPoint>> {
        let (tx, rx) = broadcast::channel(config.queue_size);
        self.subscription_tx = Some(tx.clone());

        log::info!("Modbus subscribed to {} tags", config.tag_names.len());

        Ok(rx)
    }

    async fn unsubscribe(&mut self) -> Result<()> {
        self.subscription_tx = None;
        log::info!("Modbus unsubscribed");
        Ok(())
    }

    async fn browse_nodes(&self, _root_path: Option<&str>) -> Result<Vec<NodeInfo>> {
        let state = self.inner_state.lock().await;
        let mut nodes = vec![
            NodeInfo {
                node_id: "coils".to_string(),
                node_name: "Coils (FC01)".to_string(),
                node_class: NodeClass::Object,
                data_type: None,
                description: None,
                children: Vec::new(),
            },
            NodeInfo {
                node_id: "discrete_inputs".to_string(),
                node_name: "Discrete Inputs (FC02)".to_string(),
                node_class: NodeClass::Object,
                data_type: None,
                description: None,
                children: Vec::new(),
            },
            NodeInfo {
                node_id: "holding_registers".to_string(),
                node_name: "Holding Registers (FC03)".to_string(),
                node_class: NodeClass::Object,
                data_type: None,
                description: None,
                children: Vec::new(),
            },
            NodeInfo {
                node_id: "input_registers".to_string(),
                node_name: "Input Registers (FC04)".to_string(),
                node_class: NodeClass::Object,
                data_type: None,
                description: None,
                children: Vec::new(),
            },
        ];

        if let Some(inner) = state.as_ref() {
            let mut tag_nodes = Vec::new();
            for (tag_name, tag_addr) in &inner.tag_address_map {
                tag_nodes.push(NodeInfo {
                    node_id: tag_name.clone(),
                    node_name: tag_name.clone(),
                    node_class: NodeClass::Variable,
                    data_type: Some(tag_addr.data_type.clone()),
                    description: Some(format!(
                        "{:?} at address {}",
                        tag_addr.register_type, tag_addr.address
                    )),
                    children: Vec::new(),
                });
            }
            if !tag_nodes.is_empty() {
                nodes.push(NodeInfo {
                    node_id: "configured_tags".to_string(),
                    node_name: "Configured Tags".to_string(),
                    node_class: NodeClass::Object,
                    data_type: None,
                    description: None,
                    children: tag_nodes,
                });
            }
        }

        Ok(nodes)
    }
}

pub struct ModbusProtocolFactory;

impl IndustrialProtocolFactory for ModbusProtocolFactory {
    fn create(&self, config: IndustrialProtocolConfig) -> Arc<RwLock<dyn IndustrialProtocol>> {
        Arc::new(RwLock::new(ModbusProtocol::new(config)))
    }

    fn supported_protocols(&self) -> Vec<IndustrialProtocolType> {
        vec![
            IndustrialProtocolType::ModbusTcp,
            IndustrialProtocolType::ModbusRtu,
        ]
    }
}
