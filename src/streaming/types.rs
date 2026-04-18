use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent<T> {
    pub event_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_time: chrono::DateTime<chrono::Utc>,
    pub data: T,
    pub watermark: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WindowType {
    Tumbling,
    Sliding,
    Session,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub window_type: WindowType,
    pub size: Duration,
    pub slide: Option<Duration>,
    pub gap: Option<Duration>,
    pub allowed_lateness: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowResult<K, V> {
    pub window_start: chrono::DateTime<chrono::Utc>,
    pub window_end: chrono::DateTime<chrono::Utc>,
    pub key: K,
    pub value: V,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StateType {
    KeyValue,
    List,
    Map,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub checkpoint_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub state: HashMap<String, Vec<u8>>,
    pub offsets: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub parallelism: usize,
    pub checkpoint_interval: Duration,
    pub checkpoint_interval_ms: u64,
    pub max_concurrent_checkpoints: usize,
    pub max_checkpoints: usize,
    pub backpressure_enabled: bool,
    pub backpressure_threshold: usize,
    pub exactly_once_enabled: bool,
    pub buffer_size: usize,
    pub batch_size: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            parallelism: num_cpus::get(),
            checkpoint_interval: Duration::from_secs(60),
            checkpoint_interval_ms: 60000,
            max_concurrent_checkpoints: 1,
            max_checkpoints: 10,
            backpressure_enabled: true,
            backpressure_threshold: 10000,
            exactly_once_enabled: true,
            buffer_size: 10000,
            batch_size: 100,
        }
    }
}

impl<T> StreamEvent<T> {
    pub fn new(data: T) -> Self {
        let now = chrono::Utc::now();
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: now,
            event_time: now,
            data,
            watermark: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_event_time(mut self, event_time: chrono::DateTime<chrono::Utc>) -> Self {
        self.event_time = event_time;
        self
    }

    pub fn with_watermark(mut self, watermark: chrono::DateTime<chrono::Utc>) -> Self {
        self.watermark = Some(watermark);
        self
    }

    pub fn with_metadata<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_event_new() {
        let data = 42;
        let event = StreamEvent::new(data);
        
        assert!(!event.event_id.is_empty());
        assert_eq!(event.data, 42);
        assert!(event.watermark.is_none());
        assert!(event.metadata.is_empty());
    }

    #[test]
    fn test_stream_event_builder_methods() {
        let event_time = chrono::Utc::now() - Duration::hours(1);
        let watermark = chrono::Utc::now() - Duration::minutes(5);
        
        let event = StreamEvent::new("test")
            .with_event_time(event_time)
            .with_watermark(watermark)
            .with_metadata("key", "value");
        
        assert_eq!(event.data, "test");
        assert_eq!(event.event_time, event_time);
        assert_eq!(event.watermark, Some(watermark));
        assert_eq!(event.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_stream_config_default() {
        let config = StreamConfig::default();
        
        assert_eq!(config.parallelism, num_cpus::get());
        assert_eq!(config.checkpoint_interval, Duration::from_secs(60));
        assert_eq!(config.checkpoint_interval_ms, 60000);
        assert!(config.backpressure_enabled);
        assert!(config.exactly_once_enabled);
    }

    #[test]
    fn test_window_type_equality() {
        assert_eq!(WindowType::Tumbling, WindowType::Tumbling);
        assert_eq!(WindowType::Sliding, WindowType::Sliding);
        assert_eq!(WindowType::Session, WindowType::Session);
        assert_ne!(WindowType::Tumbling, WindowType::Sliding);
    }

    #[test]
    fn test_checkpoint_serde() {
        let checkpoint = Checkpoint {
            checkpoint_id: "test-id".to_string(),
            timestamp: chrono::Utc::now(),
            state: HashMap::new(),
            offsets: HashMap::new(),
        };
        
        let serialized = serde_json::to_string(&checkpoint).unwrap();
        let deserialized: Checkpoint = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(checkpoint.checkpoint_id, deserialized.checkpoint_id);
        assert_eq!(checkpoint.state, deserialized.state);
        assert_eq!(checkpoint.offsets, deserialized.offsets);
    }
}
