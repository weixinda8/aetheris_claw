pub mod traits;
pub mod types;
pub mod influxdb;
pub mod memory;

pub use traits::*;
pub use types::*;
pub use influxdb::*;
pub use memory::*;

use crate::utils::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct TimeSeriesManager {
    backends: HashMap<TimeSeriesBackendType, Arc<dyn TimeSeriesDatabaseFactory + Send + Sync>>,
}

impl TimeSeriesManager {
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }

    pub fn register_backend(
        &mut self,
        backend_type: TimeSeriesBackendType,
        factory: Arc<dyn TimeSeriesDatabaseFactory + Send + Sync>,
    ) {
        self.backends.insert(backend_type, factory);
    }

    pub fn create_database(&self, config: TimeSeriesConfig) -> Result<Arc<RwLock<dyn TimeSeriesDatabase + Send + Sync>>> {
        if let Some(factory) = self.backends.get(&config.backend_type) {
            Ok(factory.create(config))
        } else {
            let db: Arc<RwLock<dyn TimeSeriesDatabase + Send + Sync>> =
                Arc::new(RwLock::new(InMemoryTimeSeries::new(config)));
            Ok(db)
        }
    }

    pub fn supported_backends(&self) -> Vec<TimeSeriesBackendType> {
        self.backends.keys().cloned().collect()
    }
}

impl Default for TimeSeriesManager {
    fn default() -> Self {
        Self::new()
    }
}
