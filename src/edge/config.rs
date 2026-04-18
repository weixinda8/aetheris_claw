use super::{
    CompressionLevel, FilterStrategy,
    aggregation::{AggregationFunction, TimeWindow, WindowType},
    filtering::OutlierDetectionMethod,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub stream_id: String,
    pub strategy: FilterStrategy,
    pub compression_level: CompressionLevel,
    pub outlier_method: Option<OutlierDetectionMethod>,
    pub aggregation_function: Option<AggregationFunction>,
    pub time_window: Option<TimeWindow>,
    pub window_count: Option<u32>,
    pub window_type: Option<WindowType>,
    pub enabled: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            stream_id: String::new(),
            strategy: FilterStrategy::None,
            compression_level: CompressionLevel::Medium,
            outlier_method: Some(OutlierDetectionMethod::ThreeSigma),
            aggregation_function: Some(AggregationFunction::Avg),
            time_window: Some(TimeWindow::Minutes),
            window_count: Some(1),
            window_type: Some(WindowType::Tumbling),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub global_strategy: FilterStrategy,
    pub stream_configs: HashMap<String, StreamConfig>,
    pub hot_reload_enabled: bool,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            global_strategy: FilterStrategy::None,
            stream_configs: HashMap::new(),
            hot_reload_enabled: true,
            last_updated: chrono::Utc::now(),
        }
    }
}

pub struct FilterConfigManager {
    config: Arc<DashMap<String, FilterConfig>>,
}

impl FilterConfigManager {
    pub fn new() -> Self {
        Self {
            config: Arc::new(DashMap::new()),
        }
    }

    pub fn get_config(&self, key: &str) -> Option<FilterConfig> {
        self.config.get(key).map(|c| c.value().clone())
    }

    pub fn set_config(&self, key: String, config: FilterConfig) {
        self.config.insert(key, config);
    }

    pub fn get_stream_config(&self, config_key: &str, stream_id: &str) -> Option<StreamConfig> {
        self.config.get(config_key).and_then(|c| {
            c.stream_configs.get(stream_id).cloned().or_else(|| {
                Some(StreamConfig {
                    stream_id: stream_id.to_string(),
                    strategy: c.global_strategy,
                    ..Default::default()
                })
            })
        })
    }

    pub fn update_stream_config(&self, config_key: &str, stream_config: StreamConfig) -> bool {
        if let Some(mut config) = self.config.get_mut(config_key) {
            config
                .stream_configs
                .insert(stream_config.stream_id.clone(), stream_config);
            config.last_updated = chrono::Utc::now();
            true
        } else {
            false
        }
    }

    pub fn create_config(&self, key: String, config: FilterConfig) {
        self.config.insert(key, config);
    }

    pub fn delete_config(&self, key: &str) -> bool {
        self.config.remove(key).is_some()
    }

    pub fn list_configs(&self) -> Vec<(String, FilterConfig)> {
        self.config
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
}

impl Default for FilterConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
