pub mod connection;
pub mod mock;
pub mod modbus;
pub mod opcua;
pub mod traits;
pub mod types;

pub use connection::*;
pub use mock::*;
pub use modbus::*;
pub use opcua::*;
pub use traits::*;
pub use types::*;

use crate::utils::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct IndustrialProtocolManager {
    protocols:
        std::collections::HashMap<IndustrialProtocolType, Arc<dyn IndustrialProtocolFactory>>,
}

impl IndustrialProtocolManager {
    pub fn new() -> Self {
        Self {
            protocols: std::collections::HashMap::new(),
        }
    }

    pub fn register_factory(&mut self, factory: Arc<dyn IndustrialProtocolFactory>) {
        for protocol_type in factory.supported_protocols() {
            self.protocols.insert(protocol_type, factory.clone());
        }
    }

    pub fn create_protocol(
        &self,
        config: IndustrialProtocolConfig,
    ) -> Result<Arc<RwLock<dyn IndustrialProtocol>>> {
        if let Some(factory) = self.protocols.get(&config.protocol_type) {
            Ok(factory.create(config))
        } else {
            Err(crate::utils::AetherisError::Protocol(format!(
                "Protocol {:?} not supported",
                config.protocol_type
            )))
        }
    }

    pub fn supported_protocols(&self) -> Vec<IndustrialProtocolType> {
        self.protocols.keys().cloned().collect()
    }
}

impl Default for IndustrialProtocolManager {
    fn default() -> Self {
        Self::new()
    }
}
