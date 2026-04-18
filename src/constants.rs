pub const DEFAULT_SERVER_HOST: &str = "127.0.0.1";
pub const DEFAULT_SERVER_PORT: u16 = 3000;
pub const DEFAULT_JWT_EXPIRATION_HOURS: i64 = 24;
pub const DEFAULT_JWT_ISSUER: &str = "aetheris-engine";
pub const DEFAULT_JWT_SECRET_KEY: &str = "aetheris-secret-key-change-in-production";
pub const DEFAULT_OTEL_SERVICE_NAME: &str = "aetheris";
pub const DEFAULT_OTEL_OTLP_ENDPOINT: &str = "http://localhost:4317";
pub const DEFAULT_PROMETHEUS_PORT: u16 = 9090;
pub const DEFAULT_PROMETHEUS_PATH: &str = "/metrics";
pub const DEFAULT_LLM_PROVIDER: &str = "mock";
pub const DEFAULT_LLM_MODEL: &str = "gpt-4";
pub const DEFAULT_LLM_TEMPERATURE: f32 = 0.7;
pub const DEFAULT_LLM_MAX_TOKENS: u32 = 2000;
pub const DEFAULT_LLM_TIMEOUT_SECONDS: u32 = 30;
pub const MIN_PORT_NUMBER: u16 = 1024;
pub const MAX_PORT_NUMBER: u16 = 65535;

pub const DEFAULT_MODBUS_TCP_PORT: u16 = 502;
pub const DEFAULT_OPC_UA_PORT: u16 = 4840;
pub const DEFAULT_S7_PORT: u16 = 102;
pub const DEFAULT_PROTOCOL_RECONNECT_INTERVAL_SECONDS: u64 = 5;
pub const DEFAULT_PROTOCOL_TIMEOUT_SECONDS: u64 = 30;
pub const DEFAULT_PROTOCOL_POLL_INTERVAL_MILLIS: u64 = 1000;

pub const DEFAULT_STREAM_BUFFER_SIZE: usize = 1024;
pub const DEFAULT_STREAM_WINDOW_SIZE: usize = 100;
pub const DEFAULT_STREAM_BATCH_SIZE: usize = 100;
pub const DEFAULT_STREAM_PROCESS_TIMEOUT_SECONDS: u64 = 60;
pub const DEFAULT_STREAM_PARALLELISM: usize = 4;

pub const DEFAULT_TSDB_TYPE: &str = "influxdb";
pub const DEFAULT_TSDB_HOST: &str = "127.0.0.1";
pub const DEFAULT_TSDB_PORT: u16 = 8086;
pub const DEFAULT_TSDB_DATABASE: &str = "aetheris";
pub const DEFAULT_TSDB_RETENTION_POLICY: &str = "autogen";
pub const DEFAULT_TSDB_WRITE_BATCH_SIZE: usize = 1000;
pub const DEFAULT_TSDB_WRITE_INTERVAL_SECONDS: u64 = 5;
pub const DEFAULT_TSDB_CONNECTION_POOL_SIZE: usize = 10;

pub const DEFAULT_CLUSTER_MODE: bool = false;
pub const DEFAULT_CLUSTER_NODE_ID: &str = "node-001";
pub const DEFAULT_CLUSTER_ETCD_ENDPOINTS: &str = "http://127.0.0.1:2379";
pub const DEFAULT_CLUSTER_HEARTBEAT_INTERVAL_SECONDS: u64 = 5;
pub const DEFAULT_CLUSTER_LEASE_TTL_SECONDS: u64 = 15;
pub const DEFAULT_CLUSTER_SYNC_INTERVAL_SECONDS: u64 = 10;
pub const JWT_SECRET_KEY_LENGTH: usize = 32;
pub const ENVIRONMENT_PRODUCTION: &str = "production";
pub const ENVIRONMENT_DEVELOPMENT: &str = "development";

pub const CACHE_DEFAULT_TTL_SECONDS: u64 = 3600;
pub const CACHE_MAX_SIZE: u64 = 1000;
pub const CACHE_CLEANUP_INTERVAL_SECONDS: u64 = 300;

pub const PERFORMANCE_METRIC_LATENCY_THRESHOLD_MS: f64 = 1000.0;
pub const PERFORMANCE_METRIC_ERROR_RATE_THRESHOLD: f64 = 0.05;
pub const PERFORMANCE_METRIC_CACHE_HIT_RATE_THRESHOLD: f64 = 0.8;
pub const PERFORMANCE_ALERT_THRESHOLD_MULTIPLIER_WARNING: f64 = 1.5;
pub const PERFORMANCE_ALERT_THRESHOLD_MULTIPLIER_CRITICAL: f64 = 2.0;

pub const WASM_DEFAULT_MEMORY_LIMIT_BYTES: u64 = 100 * 1024 * 1024;
pub const WASM_DEFAULT_CPU_LIMIT_PERCENT: f64 = 50.0;
pub const WASM_DEFAULT_EXECUTION_TIMEOUT_SECONDS: u64 = 30;
pub const WASM_POOL_DEFAULT_SIZE: usize = 10;
pub const WASM_POOL_MAX_SIZE: usize = 100;

pub const PLUGIN_DEFAULT_TIMEOUT_SECONDS: u64 = 60;
pub const PLUGIN_HEALTH_CHECK_INTERVAL_SECONDS: u64 = 300;
pub const PLUGIN_MAX_RETRY_COUNT: u32 = 3;

pub const SECURITY_RATE_LIMIT_DEFAULT_REQUESTS: u64 = 100;
pub const SECURITY_RATE_LIMIT_DEFAULT_WINDOW_SECONDS: u64 = 60;
pub const SECURITY_AUDIT_LOG_RETENTION_DAYS: u64 = 90;
pub const SECURITY_SESSION_TIMEOUT_SECONDS: u64 = 3600;

pub const AI_RECOMMENDATION_DEFAULT_TOP_N: usize = 10;
pub const AI_RECOMMENDATION_MIN_CONFIDENCE: f64 = 0.5;
pub const AI_MODEL_TRAINING_INTERVAL_HOURS: u64 = 24;
pub const AI_BEHAVIOR_LOG_RETENTION_DAYS: u64 = 30;

pub const OCI_REGISTRY_DEFAULT_TIMEOUT_SECONDS: u64 = 300;
pub const OCI_REGISTRY_CACHE_TTL_SECONDS: u64 = 3600;
pub const OCI_REGISTRY_MAX_LAYER_SIZE_BYTES: u64 = 100 * 1024 * 1024;

pub const SKILL_MARKETPLACE_DEFAULT_PAGE_SIZE: usize = 20;
pub const SKILL_MARKETPLACE_MAX_SEARCH_RESULTS: usize = 100;
pub const SKILL_RATING_MIN: u32 = 1;
pub const SKILL_RATING_MAX: u32 = 5;

pub const PERSONA_MARKETPLACE_DEFAULT_PAGE_SIZE: usize = 20;
pub const PERSONA_MARKETPLACE_MAX_SEARCH_RESULTS: usize = 100;
pub const PERSONA_RATING_MIN: u32 = 1;
pub const PERSONA_RATING_MAX: u32 = 5;
pub const PERSONA_MAX_VERSIONS: usize = 50;
pub const PERSONA_NAME_MAX_LENGTH: usize = 100;
pub const PERSONA_DESCRIPTION_MAX_LENGTH: usize = 2000;
pub const PERSONA_MAX_TAGS: usize = 20;
pub const PERSONA_TAG_MAX_LENGTH: usize = 50;
pub const PERSONA_REVIEW_MAX_LENGTH: usize = 5000;
pub const PERSONA_REVIEW_TITLE_MAX_LENGTH: usize = 200;

pub const SANDBOX_DEFAULT_SECURITY_LEVEL: u8 = 2;
pub const SANDBOX_MAX_CONCURRENT_INSTANCES: usize = 100;
pub const SANDBOX_AUDIT_LOG_RETENTION_DAYS: u64 = 90;
pub const SANDBOX_DEFAULT_CPU_LIMIT_PERCENT: f64 = 50.0;
pub const SANDBOX_DEFAULT_MEMORY_LIMIT_BYTES: u64 = 512 * 1024 * 1024;
pub const SANDBOX_DEFAULT_DISK_IO_LIMIT_BYTES_PER_SECOND: u64 = 10 * 1024 * 1024;
pub const SANDBOX_DEFAULT_NETWORK_BANDWIDTH_LIMIT_BYTES_PER_SECOND: u64 = 10 * 1024 * 1024;
pub const SANDBOX_DEFAULT_MAX_PROCESSES: usize = 100;
pub const SANDBOX_HEARTBEAT_INTERVAL_SECONDS: u64 = 10;
pub const SANDBOX_TIMEOUT_SECONDS: u64 = 300;

pub const SOUL_PERSONA_EVOLUTION_INTERVAL_HOURS: u64 = 168;
pub const SOUL_MAX_PERSONAS: usize = 10;
pub const SOUL_DEFAULT_PERSONA_NAME: &str = "default";

pub const CDN_CACHE_TTL_SECONDS: u64 = 86400;
pub const CDN_MAX_CACHE_SIZE_BYTES: u64 = 10 * 1024 * 1024 * 1024;

pub const SMART_PRELOAD_MIN_CONFIDENCE: f64 = 0.7;
pub const SMART_PRELOAD_MAX_ITEMS: usize = 20;
pub const SMART_PRELOAD_HISTORY_RETENTION_DAYS: u64 = 7;

pub const CONFIG_VERSION_MAX_VERSIONS: usize = 100;
pub const CONFIG_AUTO_SAVE_INTERVAL_SECONDS: u64 = 300;

pub const BENCHMARK_DEFAULT_ITERATIONS: u32 = 100;
pub const BENCHMARK_WARMUP_ITERATIONS: u32 = 10;

pub const DEFAULT_TIMEOUT_MILLIS: u64 = 5000;
pub const DEFAULT_RETRY_COUNT: u32 = 3;
pub const DEFAULT_RETRY_DELAY_MILLIS: u64 = 1000;

pub const ONE_SECOND_MILLIS: u64 = 1000;
pub const ONE_MINUTE_SECONDS: u64 = 60;
pub const ONE_HOUR_SECONDS: u64 = 3600;
pub const ONE_DAY_SECONDS: u64 = 86400;
pub const ONE_WEEK_SECONDS: u64 = 604800;
pub const ONE_KB_BYTES: u64 = 1024;
pub const ONE_MB_BYTES: u64 = 1024 * 1024;
pub const ONE_GB_BYTES: u64 = 1024 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_server_values() {
        assert_eq!(DEFAULT_SERVER_HOST, "127.0.0.1");
        assert_eq!(DEFAULT_SERVER_PORT, 3000);
    }

    #[test]
    fn test_jwt_configuration() {
        assert_eq!(DEFAULT_JWT_EXPIRATION_HOURS, 24);
        assert_eq!(DEFAULT_JWT_ISSUER, "aetheris-engine");
        assert_eq!(
            DEFAULT_JWT_SECRET_KEY,
            "aetheris-secret-key-change-in-production"
        );
        assert_eq!(JWT_SECRET_KEY_LENGTH, 32);
    }

    #[test]
    fn test_port_range() {
        assert_eq!(MIN_PORT_NUMBER, 1024);
        assert_eq!(MAX_PORT_NUMBER, 65535);
    }

    #[test]
    fn test_llm_defaults() {
        assert_eq!(DEFAULT_LLM_PROVIDER, "mock");
        assert_eq!(DEFAULT_LLM_MODEL, "gpt-4");
        assert!(DEFAULT_LLM_TEMPERATURE >= 0.0 && DEFAULT_LLM_TEMPERATURE <= 1.0);
    }

    #[test]
    fn test_cache_configuration() {
        assert_eq!(CACHE_DEFAULT_TTL_SECONDS, 3600);
        assert_eq!(CACHE_MAX_SIZE, 1000);
        assert_eq!(CACHE_CLEANUP_INTERVAL_SECONDS, 300);
    }

    #[test]
    fn test_performance_thresholds() {
        assert!(PERFORMANCE_METRIC_LATENCY_THRESHOLD_MS > 0.0);
        assert!(PERFORMANCE_METRIC_ERROR_RATE_THRESHOLD > 0.0);
        assert!(PERFORMANCE_METRIC_CACHE_HIT_RATE_THRESHOLD > 0.0);
    }

    #[test]
    fn test_wasm_configuration() {
        assert!(WASM_DEFAULT_MEMORY_LIMIT_BYTES > 0);
        assert!(WASM_DEFAULT_CPU_LIMIT_PERCENT > 0.0 && WASM_DEFAULT_CPU_LIMIT_PERCENT <= 100.0);
        assert_eq!(WASM_POOL_DEFAULT_SIZE, 10);
        assert!(WASM_POOL_MAX_SIZE >= WASM_POOL_DEFAULT_SIZE);
    }

    #[test]
    fn test_security_configuration() {
        assert!(SECURITY_RATE_LIMIT_DEFAULT_REQUESTS > 0);
        assert!(SECURITY_AUDIT_LOG_RETENTION_DAYS > 0);
        assert!(SECURITY_SESSION_TIMEOUT_SECONDS > 0);
    }

    #[test]
    fn test_time_constants() {
        assert_eq!(ONE_SECOND_MILLIS, 1000);
        assert_eq!(ONE_MINUTE_SECONDS, 60);
        assert_eq!(ONE_HOUR_SECONDS, 3600);
        assert_eq!(ONE_DAY_SECONDS, 86400);
        assert_eq!(ONE_WEEK_SECONDS, 604800);
    }

    #[test]
    fn test_byte_constants() {
        assert_eq!(ONE_KB_BYTES, 1024);
        assert_eq!(ONE_MB_BYTES, 1024 * 1024);
        assert_eq!(ONE_GB_BYTES, 1024 * 1024 * 1024);
    }

    #[test]
    fn test_sandbox_configuration() {
        assert_eq!(SANDBOX_DEFAULT_SECURITY_LEVEL, 2);
        assert!(SANDBOX_MAX_CONCURRENT_INSTANCES > 0);
        assert!(SANDBOX_AUDIT_LOG_RETENTION_DAYS > 0);
    }

    #[test]
    fn test_marketplace_configuration() {
        assert_eq!(SKILL_MARKETPLACE_DEFAULT_PAGE_SIZE, 20);
        assert_eq!(SKILL_RATING_MIN, 1);
        assert_eq!(SKILL_RATING_MAX, 5);
        assert_eq!(PERSONA_MARKETPLACE_DEFAULT_PAGE_SIZE, 20);
        assert_eq!(PERSONA_RATING_MIN, 1);
        assert_eq!(PERSONA_RATING_MAX, 5);
    }
}
