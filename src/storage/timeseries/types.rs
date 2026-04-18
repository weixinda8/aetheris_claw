use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub measurement: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tags: HashMap<String, String>,
    pub fields: HashMap<String, TimeSeriesValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeSeriesValue {
    Boolean(bool),
    Int64(i64),
    UInt64(u64),
    Float64(f64),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesQuery {
    pub measurement: String,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub tags: Option<HashMap<String, Vec<String>>>,
    pub fields: Option<Vec<String>>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub order: Option<QueryOrder>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueryOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub name: String,
    pub duration: Duration,
    pub shard_duration: Option<Duration>,
    pub replication: Option<u32>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownsamplingRule {
    pub name: String,
    pub source_measurement: String,
    pub target_measurement: String,
    pub interval: Duration,
    pub aggregation: AggregationType,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AggregationType {
    Mean,
    Sum,
    Min,
    Max,
    Count,
    First,
    Last,
    Median,
    StdDev,
    Variance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesConfig {
    pub backend_type: TimeSeriesBackendType,
    pub endpoint: String,
    pub port: u16,
    pub database: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub batch_size: usize,
    pub max_retries: u32,
    pub retry_interval: Duration,
    pub retention_policies: Vec<RetentionPolicy>,
    pub downsampling_rules: Vec<DownsamplingRule>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TimeSeriesBackendType {
    InMemory,
    InfluxDB,
    Prometheus,
    VictoriaMetrics,
}

impl Default for TimeSeriesConfig {
    fn default() -> Self {
        Self {
            backend_type: TimeSeriesBackendType::InMemory,
            endpoint: "127.0.0.1".to_string(),
            port: 8086,
            database: "aetheris".to_string(),
            username: None,
            password: None,
            token: None,
            batch_size: 1000,
            max_retries: 3,
            retry_interval: Duration::from_millis(100),
            retention_policies: vec![RetentionPolicy {
                name: "autogen".to_string(),
                duration: Duration::from_secs(0),
                shard_duration: None,
                replication: None,
                is_default: true,
            }],
            downsampling_rules: Vec::new(),
        }
    }
}

impl TimeSeriesPoint {
    pub fn new(measurement: String, timestamp: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            measurement,
            timestamp,
            tags: HashMap::new(),
            fields: HashMap::new(),
        }
    }

    pub fn add_tag<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.tags.insert(key.into(), value.into());
        self
    }

    pub fn add_field<K>(mut self, key: K, value: TimeSeriesValue) -> Self
    where
        K: Into<String>,
    {
        self.fields.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_series_point_new() {
        let now = chrono::Utc::now();
        let point = TimeSeriesPoint::new("temperature".to_string(), now);
        
        assert_eq!(point.measurement, "temperature");
        assert_eq!(point.timestamp, now);
        assert!(point.tags.is_empty());
        assert!(point.fields.is_empty());
    }

    #[test]
    fn test_time_series_point_builder() {
        let now = chrono::Utc::now();
        let point = TimeSeriesPoint::new("temperature".to_string(), now)
            .add_tag("sensor", "sensor_001")
            .add_tag("location", "factory")
            .add_field("value", TimeSeriesValue::Float64(25.5))
            .add_field("unit", TimeSeriesValue::String("Celsius".to_string()));
        
        assert_eq!(point.tags.len(), 2);
        assert_eq!(point.tags.get("sensor"), Some(&"sensor_001".to_string()));
        assert_eq!(point.tags.get("location"), Some(&"factory".to_string()));
        assert_eq!(point.fields.len(), 2);
        assert!(matches!(point.fields.get("value"), Some(&TimeSeriesValue::Float64(25.5))));
    }

    #[test]
    fn test_time_series_config_default() {
        let config = TimeSeriesConfig::default();
        
        assert_eq!(config.backend_type, TimeSeriesBackendType::InMemory);
        assert_eq!(config.endpoint, "127.0.0.1");
        assert_eq!(config.port, 8086);
        assert_eq!(config.database, "aetheris");
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_interval, Duration::from_millis(100));
        assert_eq!(config.retention_policies.len(), 1);
        assert!(config.downsampling_rules.is_empty());
    }

    #[test]
    fn test_time_series_value_variants() {
        let bool_val = TimeSeriesValue::Boolean(true);
        let int_val = TimeSeriesValue::Int64(42);
        let uint_val = TimeSeriesValue::UInt64(100);
        let float_val = TimeSeriesValue::Float64(3.14);
        let string_val = TimeSeriesValue::String("test".to_string());
        
        assert!(matches!(bool_val, TimeSeriesValue::Boolean(true)));
        assert!(matches!(int_val, TimeSeriesValue::Int64(42)));
        assert!(matches!(uint_val, TimeSeriesValue::UInt64(100)));
        assert!(matches!(float_val, TimeSeriesValue::Float64(3.14)));
        assert!(matches!(string_val, TimeSeriesValue::String(s) if s == "test"));
    }

    #[test]
    fn test_query_order() {
        assert_eq!(QueryOrder::Ascending, QueryOrder::Ascending);
        assert_eq!(QueryOrder::Descending, QueryOrder::Descending);
        assert_ne!(QueryOrder::Ascending, QueryOrder::Descending);
    }

    #[test]
    fn test_aggregation_type() {
        let types = [
            AggregationType::Mean,
            AggregationType::Sum,
            AggregationType::Min,
            AggregationType::Max,
            AggregationType::Count,
            AggregationType::First,
            AggregationType::Last,
            AggregationType::Median,
            AggregationType::StdDev,
            AggregationType::Variance,
        ];
        
        for (i, t1) in types.iter().enumerate() {
            for (j, t2) in types.iter().enumerate() {
                if i == j {
                    assert_eq!(t1, t2);
                } else {
                    assert_ne!(t1, t2);
                }
            }
        }
    }

    #[test]
    fn test_time_series_backend_type() {
        let types = [
            TimeSeriesBackendType::InMemory,
            TimeSeriesBackendType::InfluxDB,
            TimeSeriesBackendType::Prometheus,
            TimeSeriesBackendType::VictoriaMetrics,
        ];
        
        for (i, t1) in types.iter().enumerate() {
            for (j, t2) in types.iter().enumerate() {
                if i == j {
                    assert_eq!(t1, t2);
                } else {
                    assert_ne!(t1, t2);
                }
            }
        }
    }

    #[test]
    fn test_retention_policy() {
        let policy = RetentionPolicy {
            name: "test-policy".to_string(),
            duration: Duration::from_hours(24),
            shard_duration: Some(Duration::from_hours(1)),
            replication: Some(3),
            is_default: false,
        };
        
        assert_eq!(policy.name, "test-policy");
        assert_eq!(policy.duration, Duration::from_hours(24));
        assert_eq!(policy.shard_duration, Some(Duration::from_hours(1)));
        assert_eq!(policy.replication, Some(3));
        assert!(!policy.is_default);
    }

    #[test]
    fn test_downsampling_rule() {
        let rule = DownsamplingRule {
            name: "test-rule".to_string(),
            source_measurement: "raw".to_string(),
            target_measurement: "downsampled".to_string(),
            interval: Duration::from_minutes(5),
            aggregation: AggregationType::Mean,
            fields: vec!["value".to_string()],
        };
        
        assert_eq!(rule.name, "test-rule");
        assert_eq!(rule.source_measurement, "raw");
        assert_eq!(rule.target_measurement, "downsampled");
        assert_eq!(rule.interval, Duration::from_minutes(5));
        assert_eq!(rule.aggregation, AggregationType::Mean);
        assert_eq!(rule.fields, vec!["value".to_string()]);
    }

    #[test]
    fn test_time_series_query_serde() {
        let query = TimeSeriesQuery {
            measurement: "temperature".to_string(),
            start_time: Some(chrono::Utc::now() - Duration::hours(1)),
            end_time: Some(chrono::Utc::now()),
            tags: Some({
                let mut tags = HashMap::new();
                tags.insert("sensor".to_string(), vec!["sensor_001".to_string()]);
                tags
            }),
            fields: Some(vec!["value".to_string()]),
            limit: Some(100),
            offset: Some(0),
            order: Some(QueryOrder::Ascending),
        };
        
        let serialized = serde_json::to_string(&query).unwrap();
        let deserialized: TimeSeriesQuery = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(query.measurement, deserialized.measurement);
        assert_eq!(query.limit, deserialized.limit);
        assert_eq!(query.offset, deserialized.offset);
        assert_eq!(query.order, deserialized.order);
    }
}
