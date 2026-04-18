pub mod onboard;
pub mod openclaw;
pub mod skill_config;
pub mod template_library;
pub mod version_control;

use crate::constants::*;
use crate::utils::{AetherisError, Result};
use base64::Engine;
use dotenvy;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub telemetry: TelemetryConfig,
    pub llm: LlmConfig,
    pub agents: Vec<ConfigAgentConfig>,
    pub industrial_protocols: IndustrialProtocolsConfig,
    pub streaming: StreamingConfig,
    pub tsdb: TsdbConfig,
    pub cluster: ClusterConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustrialProtocolsConfig {
    pub modbus_tcp_enabled: bool,
    pub modbus_tcp_host: String,
    pub modbus_tcp_port: u16,
    pub opc_ua_enabled: bool,
    pub opc_ua_host: String,
    pub opc_ua_port: u16,
    pub s7_enabled: bool,
    pub s7_host: String,
    pub s7_port: u16,
    pub s7_rack: u8,
    pub s7_slot: u8,
    pub reconnect_interval_seconds: u64,
    pub timeout_seconds: u64,
    pub poll_interval_millis: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    pub enabled: bool,
    pub buffer_size: usize,
    pub window_size: usize,
    pub batch_size: usize,
    pub process_timeout_seconds: u64,
    pub parallelism: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsdbConfig {
    pub db_type: String,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: String,
    pub retention_policy: String,
    pub write_batch_size: usize,
    pub write_interval_seconds: u64,
    pub connection_pool_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub mode: bool,
    pub node_id: String,
    pub etcd_endpoints: String,
    pub heartbeat_interval_seconds: u64,
    pub lease_ttl_seconds: u64,
    pub sync_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret_key: String,
    pub jwt_expiration_hours: i64,
    pub jwt_issuer: String,
    pub initial_admin_username: Option<String>,
    pub initial_admin_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub otel_enabled: bool,
    pub otel_service_name: String,
    pub otel_otlp_endpoint: String,
    pub prometheus_enabled: bool,
    pub prometheus_port: u16,
    pub prometheus_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub timeout_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigAgentConfig {
    pub id: String,
    pub name: String,
    pub agent_type: String,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let _ = dotenvy::dotenv();

        Self::from_env()
    }

    fn from_env() -> Result<Self> {
        Ok(Self {
            server: ServerConfig {
                host: get_env("SERVER_HOST", DEFAULT_SERVER_HOST),
                port: get_env_parse("SERVER_PORT", DEFAULT_SERVER_PORT),
            },
            auth: AuthConfig {
                jwt_secret_key: Self::get_jwt_secret()?,
                jwt_expiration_hours: get_env_parse(
                    "JWT_EXPIRATION_HOURS",
                    DEFAULT_JWT_EXPIRATION_HOURS,
                ),
                jwt_issuer: get_env("JWT_ISSUER", DEFAULT_JWT_ISSUER),
                initial_admin_username: env::var("INITIAL_ADMIN_USERNAME").ok(),
                initial_admin_password: env::var("INITIAL_ADMIN_PASSWORD").ok(),
            },
            telemetry: TelemetryConfig {
                otel_enabled: get_env_bool("OTEL_ENABLED", false),
                otel_service_name: get_env("OTEL_SERVICE_NAME", DEFAULT_OTEL_SERVICE_NAME),
                otel_otlp_endpoint: get_env("OTEL_OTLP_ENDPOINT", DEFAULT_OTEL_OTLP_ENDPOINT),
                prometheus_enabled: get_env_bool("PROMETHEUS_ENABLED", true),
                prometheus_port: get_env_parse("PROMETHEUS_PORT", DEFAULT_PROMETHEUS_PORT),
                prometheus_path: get_env("PROMETHEUS_PATH", DEFAULT_PROMETHEUS_PATH),
            },
            llm: LlmConfig {
                provider: get_env("LLM_PROVIDER", DEFAULT_LLM_PROVIDER),
                api_key: env::var("LLM_API_KEY").ok(),
                api_base: env::var("LLM_API_BASE").ok(),
                model: get_env("LLM_MODEL", DEFAULT_LLM_MODEL),
                temperature: get_env_parse("LLM_TEMPERATURE", DEFAULT_LLM_TEMPERATURE),
                max_tokens: get_env_parse("LLM_MAX_TOKENS", DEFAULT_LLM_MAX_TOKENS),
                timeout_seconds: get_env_parse("LLM_TIMEOUT_SECONDS", DEFAULT_LLM_TIMEOUT_SECONDS),
            },
            agents: Self::get_default_agents(),
            industrial_protocols: IndustrialProtocolsConfig {
                modbus_tcp_enabled: get_env_bool("MODBUS_TCP_ENABLED", false),
                modbus_tcp_host: get_env("MODBUS_TCP_HOST", DEFAULT_SERVER_HOST),
                modbus_tcp_port: get_env_parse("MODBUS_TCP_PORT", DEFAULT_MODBUS_TCP_PORT),
                opc_ua_enabled: get_env_bool("OPC_UA_ENABLED", false),
                opc_ua_host: get_env("OPC_UA_HOST", DEFAULT_SERVER_HOST),
                opc_ua_port: get_env_parse("OPC_UA_PORT", DEFAULT_OPC_UA_PORT),
                s7_enabled: get_env_bool("S7_ENABLED", false),
                s7_host: get_env("S7_HOST", DEFAULT_SERVER_HOST),
                s7_port: get_env_parse("S7_PORT", DEFAULT_S7_PORT),
                s7_rack: get_env_parse("S7_RACK", 0),
                s7_slot: get_env_parse("S7_SLOT", 1),
                reconnect_interval_seconds: get_env_parse(
                    "PROTOCOL_RECONNECT_INTERVAL_SECONDS",
                    DEFAULT_PROTOCOL_RECONNECT_INTERVAL_SECONDS,
                ),
                timeout_seconds: get_env_parse(
                    "PROTOCOL_TIMEOUT_SECONDS",
                    DEFAULT_PROTOCOL_TIMEOUT_SECONDS,
                ),
                poll_interval_millis: get_env_parse(
                    "PROTOCOL_POLL_INTERVAL_MILLIS",
                    DEFAULT_PROTOCOL_POLL_INTERVAL_MILLIS,
                ),
            },
            streaming: StreamingConfig {
                enabled: get_env_bool("STREAMING_ENABLED", false),
                buffer_size: get_env_parse("STREAM_BUFFER_SIZE", DEFAULT_STREAM_BUFFER_SIZE),
                window_size: get_env_parse("STREAM_WINDOW_SIZE", DEFAULT_STREAM_WINDOW_SIZE),
                batch_size: get_env_parse("STREAM_BATCH_SIZE", DEFAULT_STREAM_BATCH_SIZE),
                process_timeout_seconds: get_env_parse(
                    "STREAM_PROCESS_TIMEOUT_SECONDS",
                    DEFAULT_STREAM_PROCESS_TIMEOUT_SECONDS,
                ),
                parallelism: get_env_parse("STREAM_PARALLELISM", DEFAULT_STREAM_PARALLELISM),
            },
            tsdb: TsdbConfig {
                db_type: get_env("TSDB_TYPE", DEFAULT_TSDB_TYPE),
                host: get_env("TSDB_HOST", DEFAULT_TSDB_HOST),
                port: get_env_parse("TSDB_PORT", DEFAULT_TSDB_PORT),
                username: env::var("TSDB_USERNAME").ok(),
                password: env::var("TSDB_PASSWORD").ok(),
                database: get_env("TSDB_DATABASE", DEFAULT_TSDB_DATABASE),
                retention_policy: get_env("TSDB_RETENTION_POLICY", DEFAULT_TSDB_RETENTION_POLICY),
                write_batch_size: get_env_parse(
                    "TSDB_WRITE_BATCH_SIZE",
                    DEFAULT_TSDB_WRITE_BATCH_SIZE,
                ),
                write_interval_seconds: get_env_parse(
                    "TSDB_WRITE_INTERVAL_SECONDS",
                    DEFAULT_TSDB_WRITE_INTERVAL_SECONDS,
                ),
                connection_pool_size: get_env_parse(
                    "TSDB_CONNECTION_POOL_SIZE",
                    DEFAULT_TSDB_CONNECTION_POOL_SIZE,
                ),
            },
            cluster: ClusterConfig {
                mode: get_env_bool("CLUSTER_MODE", DEFAULT_CLUSTER_MODE),
                node_id: get_env("CLUSTER_NODE_ID", DEFAULT_CLUSTER_NODE_ID),
                etcd_endpoints: get_env("CLUSTER_ETCD_ENDPOINTS", DEFAULT_CLUSTER_ETCD_ENDPOINTS),
                heartbeat_interval_seconds: get_env_parse(
                    "CLUSTER_HEARTBEAT_INTERVAL_SECONDS",
                    DEFAULT_CLUSTER_HEARTBEAT_INTERVAL_SECONDS,
                ),
                lease_ttl_seconds: get_env_parse(
                    "CLUSTER_LEASE_TTL_SECONDS",
                    DEFAULT_CLUSTER_LEASE_TTL_SECONDS,
                ),
                sync_interval_seconds: get_env_parse(
                    "CLUSTER_SYNC_INTERVAL_SECONDS",
                    DEFAULT_CLUSTER_SYNC_INTERVAL_SECONDS,
                ),
            },
        })
    }

    fn get_jwt_secret() -> Result<String> {
        let environment =
            env::var("ENVIRONMENT").unwrap_or_else(|_| ENVIRONMENT_DEVELOPMENT.to_string());
        let is_production = environment == ENVIRONMENT_PRODUCTION;

        match env::var("JWT_SECRET_KEY") {
            Ok(secret) if !secret.is_empty() => {
                if secret == DEFAULT_JWT_SECRET_KEY {
                    if is_production {
                        return Err(AetherisError::Config(
                            "CRITICAL: Default JWT secret detected in production. ".to_string()
                                + "This is extremely insecure and not allowed. "
                                + "Please set a secure, unique JWT_SECRET_KEY environment variable. "
                                + "The secret must be at least 32 bytes long for production use.",
                        ));
                    } else {
                        return Err(AetherisError::Config(
                            "WARNING: Default JWT secret detected even in development. ".to_string()
                                + "Please set a JWT_SECRET_KEY environment variable or remove it "
                                + "to allow automatic random generation (development only).",
                        ));
                    }
                }

                if secret.len() < 32 {
                    return Err(AetherisError::Config(format!(
                        "JWT secret must be at least 32 bytes long. Current length: {} bytes. Please use a longer, more secure secret for cryptographic operations.",
                        secret.len()
                    )));
                }

                Ok(secret)
            }
            _ => {
                if is_production {
                    return Err(AetherisError::Config(
                        "CRITICAL: JWT_SECRET_KEY must be explicitly set in production environment. ".to_string() +
                        "Please generate a secure random secret (minimum 32 bytes) and set it as an environment variable. " +
                        "You can use tools like 'openssl rand -base64 32' to generate a secure secret."
                    ));
                }

                let mut key = [0u8; JWT_SECRET_KEY_LENGTH];
                rand::thread_rng().fill_bytes(&mut key);
                Ok(base64::engine::general_purpose::STANDARD.encode(key))
            }
        }
    }

    fn get_default_agents() -> Vec<ConfigAgentConfig> {
        vec![
            ConfigAgentConfig {
                id: "code-agent-001".to_string(),
                name: "Code Execution Agent".to_string(),
                agent_type: "code".to_string(),
            },
            ConfigAgentConfig {
                id: "data-agent-001".to_string(),
                name: "Data Processing Agent".to_string(),
                agent_type: "data".to_string(),
            },
            ConfigAgentConfig {
                id: "ops-agent-001".to_string(),
                name: "Operations Agent".to_string(),
                agent_type: "ops".to_string(),
            },
        ]
    }

    pub fn socket_addr(&self) -> SocketAddr {
        let ip: IpAddr = self
            .server
            .host
            .parse()
            .unwrap_or_else(|_| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        SocketAddr::from((ip, self.server.port))
    }

    pub fn validate(&self) -> Result<()> {
        if self.server.port < MIN_PORT_NUMBER {
            return Err(AetherisError::Config(format!(
                "Invalid port number: {}",
                self.server.port
            )));
        }

        if self.industrial_protocols.modbus_tcp_port < MIN_PORT_NUMBER {
            return Err(AetherisError::Config(format!(
                "Invalid Modbus TCP port: {}",
                self.industrial_protocols.modbus_tcp_port
            )));
        }

        if self.industrial_protocols.opc_ua_port < MIN_PORT_NUMBER {
            return Err(AetherisError::Config(format!(
                "Invalid OPC UA port: {}",
                self.industrial_protocols.opc_ua_port
            )));
        }

        if self.industrial_protocols.s7_port < MIN_PORT_NUMBER {
            return Err(AetherisError::Config(format!(
                "Invalid S7 port: {}",
                self.industrial_protocols.s7_port
            )));
        }

        if self.industrial_protocols.poll_interval_millis < 10 {
            return Err(AetherisError::Config(
                "Protocol poll interval must be at least 10ms".to_string(),
            ));
        }

        if self.streaming.buffer_size < 1 {
            return Err(AetherisError::Config(
                "Stream buffer size must be at least 1".to_string(),
            ));
        }

        if self.streaming.window_size < 1 {
            return Err(AetherisError::Config(
                "Stream window size must be at least 1".to_string(),
            ));
        }

        if self.streaming.batch_size < 1 {
            return Err(AetherisError::Config(
                "Stream batch size must be at least 1".to_string(),
            ));
        }

        if self.streaming.parallelism < 1 {
            return Err(AetherisError::Config(
                "Stream parallelism must be at least 1".to_string(),
            ));
        }

        if self.tsdb.port < MIN_PORT_NUMBER {
            return Err(AetherisError::Config(format!(
                "Invalid TSDB port: {}",
                self.tsdb.port
            )));
        }

        if self.tsdb.write_batch_size < 1 {
            return Err(AetherisError::Config(
                "TSDB write batch size must be at least 1".to_string(),
            ));
        }

        if self.tsdb.write_interval_seconds < 1 {
            return Err(AetherisError::Config(
                "TSDB write interval must be at least 1 second".to_string(),
            ));
        }

        if self.tsdb.connection_pool_size < 1 {
            return Err(AetherisError::Config(
                "TSDB connection pool size must be at least 1".to_string(),
            ));
        }

        if self.cluster.mode {
            if self.cluster.node_id.is_empty() {
                return Err(AetherisError::Config(
                    "Cluster node ID cannot be empty in cluster mode".to_string(),
                ));
            }

            if self.cluster.etcd_endpoints.is_empty() {
                return Err(AetherisError::Config(
                    "Cluster etcd endpoints cannot be empty in cluster mode".to_string(),
                ));
            }

            if self.cluster.heartbeat_interval_seconds < 1 {
                return Err(AetherisError::Config(
                    "Cluster heartbeat interval must be at least 1 second".to_string(),
                ));
            }

            if self.cluster.lease_ttl_seconds <= self.cluster.heartbeat_interval_seconds {
                return Err(AetherisError::Config(
                    "Cluster lease TTL must be greater than heartbeat interval".to_string(),
                ));
            }

            if self.cluster.sync_interval_seconds < 1 {
                return Err(AetherisError::Config(
                    "Cluster sync interval must be at least 1 second".to_string(),
                ));
            }
        }

        Ok(())
    }
}

fn get_env(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn get_env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn get_env_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .ok()
        .and_then(|v| v.to_lowercase().parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_app_config_defaults() {
        let config = AppConfig::from_env().unwrap();
        assert_eq!(config.server.host, DEFAULT_SERVER_HOST);
        assert_eq!(config.server.port, DEFAULT_SERVER_PORT);
        assert_eq!(config.llm.provider, DEFAULT_LLM_PROVIDER);
        assert_eq!(config.llm.model, DEFAULT_LLM_MODEL);
    }

    #[test]
    fn test_app_config_validate_valid_port() {
        let mut config = AppConfig::from_env().unwrap();
        config.server.port = 8080;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_app_config_validate_invalid_port_low() {
        let mut config = AppConfig::from_env().unwrap();
        config.server.port = 100;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_app_config_validate_invalid_port_high() {
        let mut config = AppConfig::from_env().unwrap();
        config.server.port = 1023;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_app_config_socket_addr() {
        let config = AppConfig::from_env().unwrap();
        let addr = config.socket_addr();
        assert_eq!(addr.port(), DEFAULT_SERVER_PORT);
    }

    #[test]
    fn test_default_agents_config() {
        let config = AppConfig::from_env().unwrap();
        assert!(!config.agents.is_empty());
        assert_eq!(config.agents.len(), 3);
    }

    #[test]
    fn test_telemetry_config_defaults() {
        let config = AppConfig::from_env().unwrap();
        assert_eq!(
            config.telemetry.otel_service_name,
            DEFAULT_OTEL_SERVICE_NAME
        );
        assert_eq!(config.telemetry.prometheus_port, DEFAULT_PROMETHEUS_PORT);
    }

    #[test]
    fn test_auth_config_defaults() {
        let config = AppConfig::from_env().unwrap();
        assert_eq!(
            config.auth.jwt_expiration_hours,
            DEFAULT_JWT_EXPIRATION_HOURS
        );
        assert_eq!(config.auth.jwt_issuer, DEFAULT_JWT_ISSUER);
    }

    #[test]
    fn test_jwt_secret_valid() {
        unsafe {
            env::remove_var("JWT_SECRET_KEY");
            env::remove_var("ENVIRONMENT");
        }
        let config = AppConfig::from_env().unwrap();
        assert!(!config.auth.jwt_secret_key.is_empty());
    }

    #[test]
    fn test_jwt_secret_too_short() {
        unsafe {
            env::set_var("JWT_SECRET_KEY", "short-key");
            env::remove_var("ENVIRONMENT");
        }
        let result = AppConfig::from_env();
        assert!(result.is_err());
        unsafe {
            env::remove_var("JWT_SECRET_KEY");
        }
    }

    #[test]
    fn test_jwt_secret_valid_min_length() {
        let long_secret = "a".repeat(32);
        unsafe {
            env::set_var("JWT_SECRET_KEY", &long_secret);
            env::remove_var("ENVIRONMENT");
        }
        let result = AppConfig::from_env();
        assert!(result.is_ok());
        unsafe {
            env::remove_var("JWT_SECRET_KEY");
        }
    }

    #[test]
    fn test_jwt_default_secret_development() {
        unsafe {
            env::set_var("JWT_SECRET_KEY", DEFAULT_JWT_SECRET_KEY);
            env::set_var("ENVIRONMENT", ENVIRONMENT_DEVELOPMENT);
        }
        let result = AppConfig::from_env();
        assert!(result.is_err());
        unsafe {
            env::remove_var("JWT_SECRET_KEY");
            env::remove_var("ENVIRONMENT");
        }
    }

    #[test]
    fn test_jwt_default_secret_production() {
        unsafe {
            env::set_var("JWT_SECRET_KEY", DEFAULT_JWT_SECRET_KEY);
            env::set_var("ENVIRONMENT", ENVIRONMENT_PRODUCTION);
        }
        let result = AppConfig::from_env();
        assert!(result.is_err());
        unsafe {
            env::remove_var("JWT_SECRET_KEY");
            env::remove_var("ENVIRONMENT");
        }
    }

    #[test]
    fn test_jwt_secret_missing_production() {
        unsafe {
            env::remove_var("JWT_SECRET_KEY");
            env::set_var("ENVIRONMENT", ENVIRONMENT_PRODUCTION);
        }
        let result = AppConfig::from_env();
        assert!(result.is_err());
        unsafe {
            env::remove_var("ENVIRONMENT");
        }
    }
}
