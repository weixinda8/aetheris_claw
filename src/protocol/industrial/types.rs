use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IndustrialProtocolType {
    OpcUa,
    ModbusTcp,
    ModbusRtu,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustrialProtocolConfig {
    pub protocol_type: IndustrialProtocolType,
    pub endpoint: String,
    pub port: u16,
    pub timeout_ms: u64,
    pub reconnect_interval_ms: u64,
    pub max_reconnect_attempts: u32,
    pub security_config: Option<SecurityConfig>,
    pub extra_config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub use_tls: bool,
    pub ca_cert_path: Option<String>,
    pub client_cert_path: Option<String>,
    pub client_key_path: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub tag_name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub value: DataValue,
    pub quality: DataQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataValue {
    Boolean(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
    String(String),
    ByteArray(Vec<u8>),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DataQuality {
    Good,
    Uncertain,
    Bad,
    BadConfigError,
    BadNotConnected,
    BadDeviceFailure,
    BadSensorFailure,
    BadOutOfService,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteRequest {
    pub tag_name: String,
    pub value: DataValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    pub tag_name: String,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionConfig {
    pub tag_names: Vec<String>,
    pub sampling_interval_ms: u32,
    pub queue_size: usize,
    pub discard_oldest: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

impl Default for IndustrialProtocolConfig {
    fn default() -> Self {
        Self {
            protocol_type: IndustrialProtocolType::OpcUa,
            endpoint: "127.0.0.1".to_string(),
            port: 4840,
            timeout_ms: 5000,
            reconnect_interval_ms: 3000,
            max_reconnect_attempts: 10,
            security_config: None,
            extra_config: HashMap::new(),
        }
    }
}

impl Default for SubscriptionConfig {
    fn default() -> Self {
        Self {
            tag_names: Vec::new(),
            sampling_interval_ms: 1000,
            queue_size: 10000,
            discard_oldest: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_industrial_protocol_type() {
        let types = [
            IndustrialProtocolType::OpcUa,
            IndustrialProtocolType::ModbusTcp,
            IndustrialProtocolType::ModbusRtu,
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
    fn test_industrial_protocol_config_default() {
        let config = IndustrialProtocolConfig::default();
        
        assert_eq!(config.protocol_type, IndustrialProtocolType::OpcUa);
        assert_eq!(config.endpoint, "127.0.0.1");
        assert_eq!(config.port, 4840);
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.reconnect_interval_ms, 3000);
        assert_eq!(config.max_reconnect_attempts, 10);
        assert!(config.security_config.is_none());
        assert!(config.extra_config.is_empty());
    }

    #[test]
    fn test_security_config() {
        let config = SecurityConfig {
            use_tls: true,
            ca_cert_path: Some("/path/to/ca.crt".to_string()),
            client_cert_path: Some("/path/to/client.crt".to_string()),
            client_key_path: Some("/path/to/client.key".to_string()),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
        };
        
        assert!(config.use_tls);
        assert_eq!(config.ca_cert_path, Some("/path/to/ca.crt".to_string()));
        assert_eq!(config.client_cert_path, Some("/path/to/client.crt".to_string()));
        assert_eq!(config.client_key_path, Some("/path/to/client.key".to_string()));
        assert_eq!(config.username, Some("user".to_string()));
        assert_eq!(config.password, Some("pass".to_string()));
    }

    #[test]
    fn test_data_value_variants() {
        let bool_val = DataValue::Boolean(true);
        let int8_val = DataValue::Int8(42);
        let int16_val = DataValue::Int16(1000);
        let int32_val = DataValue::Int32(1000000);
        let int64_val = DataValue::Int64(1000000000);
        let uint8_val = DataValue::UInt8(200);
        let uint16_val = DataValue::UInt16(50000);
        let uint32_val = DataValue::UInt32(4000000000);
        let uint64_val = DataValue::UInt64(10000000000);
        let float32_val = DataValue::Float32(3.14);
        let float64_val = DataValue::Float64(2.718);
        let string_val = DataValue::String("test".to_string());
        let byte_array_val = DataValue::ByteArray(vec![0x01, 0x02, 0x03]);
        
        assert!(matches!(bool_val, DataValue::Boolean(true)));
        assert!(matches!(int8_val, DataValue::Int8(42)));
        assert!(matches!(int16_val, DataValue::Int16(1000)));
        assert!(matches!(int32_val, DataValue::Int32(1000000)));
        assert!(matches!(int64_val, DataValue::Int64(1000000000)));
        assert!(matches!(uint8_val, DataValue::UInt8(200)));
        assert!(matches!(uint16_val, DataValue::UInt16(50000)));
        assert!(matches!(uint32_val, DataValue::UInt32(4000000000)));
        assert!(matches!(uint64_val, DataValue::UInt64(10000000000)));
        assert!(matches!(float32_val, DataValue::Float32(v) if (v - 3.14).abs() < 0.001));
        assert!(matches!(float64_val, DataValue::Float64(v) if (v - 2.718).abs() < 0.001));
        assert!(matches!(string_val, DataValue::String(s) if s == "test"));
        assert!(matches!(byte_array_val, DataValue::ByteArray(v) if v == vec![0x01, 0x02, 0x03]));
    }

    #[test]
    fn test_data_quality() {
        let qualities = [
            DataQuality::Good,
            DataQuality::Uncertain,
            DataQuality::Bad,
            DataQuality::BadConfigError,
            DataQuality::BadNotConnected,
            DataQuality::BadDeviceFailure,
            DataQuality::BadSensorFailure,
            DataQuality::BadOutOfService,
        ];
        
        for (i, q1) in qualities.iter().enumerate() {
            for (j, q2) in qualities.iter().enumerate() {
                if i == j {
                    assert_eq!(q1, q2);
                } else {
                    assert_ne!(q1, q2);
                }
            }
        }
    }

    #[test]
    fn test_data_point() {
        let now = chrono::Utc::now();
        let point = DataPoint {
            tag_name: "Temperature".to_string(),
            timestamp: now,
            value: DataValue::Float64(25.5),
            quality: DataQuality::Good,
        };
        
        assert_eq!(point.tag_name, "Temperature");
        assert_eq!(point.timestamp, now);
        assert!(matches!(point.value, DataValue::Float64(25.5)));
        assert_eq!(point.quality, DataQuality::Good);
    }

    #[test]
    fn test_write_request() {
        let request = WriteRequest {
            tag_name: "SetPoint".to_string(),
            value: DataValue::Int32(100),
        };
        
        assert_eq!(request.tag_name, "SetPoint");
        assert!(matches!(request.value, DataValue::Int32(100)));
    }

    #[test]
    fn test_write_result() {
        let success_result = WriteResult {
            tag_name: "SetPoint".to_string(),
            success: true,
            error_message: None,
        };
        
        let error_result = WriteResult {
            tag_name: "SetPoint".to_string(),
            success: false,
            error_message: Some("Write failed".to_string()),
        };
        
        assert_eq!(success_result.tag_name, "SetPoint");
        assert!(success_result.success);
        assert!(success_result.error_message.is_none());
        
        assert_eq!(error_result.tag_name, "SetPoint");
        assert!(!error_result.success);
        assert_eq!(error_result.error_message, Some("Write failed".to_string()));
    }

    #[test]
    fn test_subscription_config_default() {
        let config = SubscriptionConfig::default();
        
        assert!(config.tag_names.is_empty());
        assert_eq!(config.sampling_interval_ms, 1000);
        assert_eq!(config.queue_size, 10000);
        assert!(config.discard_oldest);
    }

    #[test]
    fn test_subscription_config_custom() {
        let config = SubscriptionConfig {
            tag_names: vec!["Temperature".to_string(), "Pressure".to_string()],
            sampling_interval_ms: 500,
            queue_size: 5000,
            discard_oldest: false,
        };
        
        assert_eq!(config.tag_names.len(), 2);
        assert_eq!(config.tag_names[0], "Temperature");
        assert_eq!(config.tag_names[1], "Pressure");
        assert_eq!(config.sampling_interval_ms, 500);
        assert_eq!(config.queue_size, 5000);
        assert!(!config.discard_oldest);
    }

    #[test]
    fn test_connection_status() {
        let statuses = [
            ConnectionStatus::Disconnected,
            ConnectionStatus::Connecting,
            ConnectionStatus::Connected,
            ConnectionStatus::Reconnecting,
            ConnectionStatus::Error,
        ];
        
        for (i, s1) in statuses.iter().enumerate() {
            for (j, s2) in statuses.iter().enumerate() {
                if i == j {
                    assert_eq!(s1, s2);
                } else {
                    assert_ne!(s1, s2);
                }
            }
        }
    }

    #[test]
    fn test_industrial_protocol_config_serde() {
        let config = IndustrialProtocolConfig {
            protocol_type: IndustrialProtocolType::ModbusTcp,
            endpoint: "192.168.1.100".to_string(),
            port: 502,
            timeout_ms: 10000,
            reconnect_interval_ms: 5000,
            max_reconnect_attempts: 5,
            security_config: Some(SecurityConfig {
                use_tls: false,
                ca_cert_path: None,
                client_cert_path: None,
                client_key_path: None,
                username: Some("admin".to_string()),
                password: Some("secret".to_string()),
            }),
            extra_config: {
                let mut map = HashMap::new();
                map.insert("unit_id".to_string(), serde_json::json!(1));
                map
            },
        };
        
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: IndustrialProtocolConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.protocol_type, deserialized.protocol_type);
        assert_eq!(config.endpoint, deserialized.endpoint);
        assert_eq!(config.port, deserialized.port);
        assert_eq!(config.timeout_ms, deserialized.timeout_ms);
    }

    #[test]
    fn test_data_point_serde() {
        let point = DataPoint {
            tag_name: "Temperature".to_string(),
            timestamp: chrono::Utc::now(),
            value: DataValue::Float64(25.5),
            quality: DataQuality::Good,
        };
        
        let serialized = serde_json::to_string(&point).unwrap();
        let deserialized: DataPoint = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(point.tag_name, deserialized.tag_name);
        assert_eq!(point.quality, deserialized.quality);
    }
}


