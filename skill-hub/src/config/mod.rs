use crate::constants::*;
use crate::utils::{Result, SkillHubError};
use serde::{Deserialize, Serialize};
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use dotenvy;
use rand::{self, RngCore};
use base64::{self, Engine};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub qdrant: QdrantConfig,
    pub auth: AuthConfig,
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: u64,
    pub idle_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    pub url: String,
    pub collection_name: String,
    pub vector_size: usize,
    pub distance: String,
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
            database: DatabaseConfig {
                url: get_env("DATABASE_URL", "postgresql://aetheris_skill_hub:aetheris_skill_hub_password@localhost:5432/aetheris_skill_hub"),
                max_connections: get_env_parse("DATABASE_MAX_CONNECTIONS", 20),
                min_connections: get_env_parse("DATABASE_MIN_CONNECTIONS", 5),
                connect_timeout: get_env_parse("DATABASE_CONNECT_TIMEOUT", 10),
                idle_timeout: get_env_parse("DATABASE_IDLE_TIMEOUT", 600),
            },
            qdrant: QdrantConfig {
                url: get_env("QDRANT_URL", "http://localhost:6333"),
                collection_name: get_env("QDRANT_COLLECTION_NAME", QDRANT_DEFAULT_COLLECTION_NAME),
                vector_size: get_env_parse("QDRANT_VECTOR_SIZE", QDRANT_VECTOR_SIZE),
                distance: get_env("QDRANT_DISTANCE", QDRANT_DEFAULT_DISTANCE),
            },
            auth: AuthConfig {
                jwt_secret_key: Self::get_jwt_secret()?,
                jwt_expiration_hours: get_env_parse("JWT_EXPIRATION_HOURS", DEFAULT_JWT_EXPIRATION_HOURS),
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
        })
    }
    
    fn get_jwt_secret() -> Result<String> {
        match env::var("JWT_SECRET_KEY") {
            Ok(secret) if !secret.is_empty() => {
                if secret == DEFAULT_JWT_SECRET_KEY
                    && env::var("ENVIRONMENT").unwrap_or_default() == ENVIRONMENT_PRODUCTION
                {
                    return Err(SkillHubError::Config("Default JWT secret cannot be used in production".to_string()));
                }
                Ok(secret)
            }
            _ => {
                if env::var("ENVIRONMENT").unwrap_or_default() == ENVIRONMENT_PRODUCTION {
                    return Err(SkillHubError::Config("JWT_SECRET_KEY must be set in production".to_string()));
                }
                
                let mut key = [0u8; JWT_SECRET_KEY_LENGTH];
                rand::thread_rng().fill_bytes(&mut key);
                Ok(base64::engine::general_purpose::STANDARD.encode(key))
            }
        }
    }
    
    pub fn socket_addr(&self) -> SocketAddr {
        let ip: IpAddr = self.server.host.parse().unwrap_or_else(|_| {
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
        });
        SocketAddr::from((ip, self.server.port))
    }
    
    pub fn validate(&self) -> Result<()> {
        if self.server.port < MIN_PORT_NUMBER || self.server.port > MAX_PORT_NUMBER {
            return Err(SkillHubError::Config(format!("Invalid port number: {}", self.server.port)));
        }
        
        if self.qdrant.vector_size == 0 {
            return Err(SkillHubError::Config("Qdrant vector size cannot be zero".to_string()));
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

    #[test]
    fn test_app_config_defaults() {
        let config = AppConfig::from_env().unwrap();
        assert_eq!(config.server.host, DEFAULT_SERVER_HOST);
        assert_eq!(config.server.port, DEFAULT_SERVER_PORT);
        assert_eq!(config.qdrant.collection_name, QDRANT_DEFAULT_COLLECTION_NAME);
        assert_eq!(config.qdrant.vector_size, QDRANT_VECTOR_SIZE);
        assert_eq!(config.qdrant.distance, QDRANT_DEFAULT_DISTANCE);
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
    fn test_telemetry_config_defaults() {
        let config = AppConfig::from_env().unwrap();
        assert_eq!(config.telemetry.otel_service_name, DEFAULT_OTEL_SERVICE_NAME);
        assert_eq!(config.telemetry.prometheus_port, DEFAULT_PROMETHEUS_PORT);
    }

    #[test]
    fn test_auth_config_defaults() {
        let config = AppConfig::from_env().unwrap();
        assert_eq!(config.auth.jwt_expiration_hours, DEFAULT_JWT_EXPIRATION_HOURS);
        assert_eq!(config.auth.jwt_issuer, DEFAULT_JWT_ISSUER);
    }
}
