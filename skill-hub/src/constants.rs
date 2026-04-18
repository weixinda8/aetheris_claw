pub const DEFAULT_SERVER_HOST: &str = "127.0.0.1";
pub const DEFAULT_SERVER_PORT: u16 = 8080;
pub const DEFAULT_JWT_EXPIRATION_HOURS: i64 = 24;
pub const DEFAULT_JWT_ISSUER: &str = "aetheris-skill-hub";
pub const DEFAULT_JWT_SECRET_KEY: &str = "aetheris-skill-hub-secret-key-change-in-production";
pub const DEFAULT_OTEL_SERVICE_NAME: &str = "aetheris-skill-hub";
pub const DEFAULT_OTEL_OTLP_ENDPOINT: &str = "http://localhost:4317";
pub const DEFAULT_PROMETHEUS_PORT: u16 = 9090;
pub const DEFAULT_PROMETHEUS_PATH: &str = "/metrics";
pub const MIN_PORT_NUMBER: u16 = 1024;
pub const MAX_PORT_NUMBER: u16 = 65535;
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

pub const SKILL_DEFAULT_PAGE_SIZE: usize = 20;
pub const SKILL_MAX_SEARCH_RESULTS: usize = 100;
pub const SKILL_RATING_MIN: u32 = 1;
pub const SKILL_RATING_MAX: u32 = 5;
pub const SKILL_NAME_MAX_LENGTH: usize = 100;
pub const SKILL_DESCRIPTION_MAX_LENGTH: usize = 2000;
pub const SKILL_MAX_TAGS: usize = 20;
pub const SKILL_TAG_MAX_LENGTH: usize = 50;
pub const SKILL_REVIEW_MAX_LENGTH: usize = 5000;
pub const SKILL_REVIEW_TITLE_MAX_LENGTH: usize = 200;
pub const SKILL_MAX_VERSIONS: usize = 50;

pub const QDRANT_DEFAULT_COLLECTION_NAME: &str = "skills";
pub const QDRANT_VECTOR_SIZE: usize = 1536;
pub const QDRANT_DEFAULT_DISTANCE: &str = "Cosine";

pub const ONE_SECOND_MILLIS: u64 = 1000;
pub const ONE_MINUTE_SECONDS: u64 = 60;
pub const ONE_HOUR_SECONDS: u64 = 3600;
pub const ONE_DAY_SECONDS: u64 = 86400;
pub const ONE_WEEK_SECONDS: u64 = 604800;
pub const ONE_KB_BYTES: u64 = 1024;
pub const ONE_MB_BYTES: u64 = 1024 * 1024;
pub const ONE_GB_BYTES: u64 = 1024 * 1024 * 1024;

pub const DEFAULT_TIMEOUT_MILLIS: u64 = 5000;
pub const DEFAULT_RETRY_COUNT: u32 = 3;
pub const DEFAULT_RETRY_DELAY_MILLIS: u64 = 1000;

pub const AUDIT_STAGE_AUTOMATED_SCAN: &str = "automated_scan";
pub const AUDIT_STAGE_JUNIOR_REVIEW: &str = "junior_review";
pub const AUDIT_STAGE_SENIOR_REVIEW: &str = "senior_review";
pub const AUDIT_STAGE_COMPLETE: &str = "complete";

pub const AUDIT_STATUS_IN_PROGRESS: &str = "in_progress";
pub const AUDIT_STATUS_APPROVED: &str = "approved";
pub const AUDIT_STATUS_REJECTED: &str = "rejected";
pub const AUDIT_STATUS_CHANGES_REQUESTED: &str = "changes_requested";

pub const SKILL_STATUS_DRAFT: &str = "draft";
pub const SKILL_STATUS_PENDING: &str = "pending";
pub const SKILL_STATUS_PUBLISHED: &str = "published";
pub const SKILL_STATUS_DEPRECATED: &str = "deprecated";
pub const SKILL_STATUS_BLOCKED: &str = "blocked";

pub const PERMISSION_PUBLIC: &str = "Public";
pub const PERMISSION_INTERNAL: &str = "Internal";
pub const PERMISSION_RESTRICTED: &str = "Restricted";
pub const PERMISSION_ADMIN: &str = "Admin";

pub const AUDIT_ACTION_APPROVE: &str = "approve";
pub const AUDIT_ACTION_REJECT: &str = "reject";
pub const AUDIT_ACTION_REQUEST_CHANGES: &str = "request_changes";

pub const AUDIT_SCAN_MIN_SCORE: f64 = 70.0;
pub const AUDIT_MAX_WAIT_TIME_SECONDS: i64 = 86400;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_server_values() {
        assert_eq!(DEFAULT_SERVER_HOST, "127.0.0.1");
        assert_eq!(DEFAULT_SERVER_PORT, 8080);
    }

    #[test]
    fn test_jwt_configuration() {
        assert_eq!(DEFAULT_JWT_EXPIRATION_HOURS, 24);
        assert_eq!(DEFAULT_JWT_ISSUER, "aetheris-skill-hub");
        assert_eq!(DEFAULT_JWT_SECRET_KEY, "aetheris-skill-hub-secret-key-change-in-production");
        assert_eq!(JWT_SECRET_KEY_LENGTH, 32);
    }

    #[test]
    fn test_port_range() {
        assert_eq!(MIN_PORT_NUMBER, 1024);
        assert_eq!(MAX_PORT_NUMBER, 65535);
    }

    #[test]
    fn test_skill_configuration() {
        assert_eq!(SKILL_DEFAULT_PAGE_SIZE, 20);
        assert_eq!(SKILL_RATING_MIN, 1);
        assert_eq!(SKILL_RATING_MAX, 5);
    }

    #[test]
    fn test_qdrant_configuration() {
        assert_eq!(QDRANT_DEFAULT_COLLECTION_NAME, "skills");
        assert_eq!(QDRANT_VECTOR_SIZE, 1536);
        assert_eq!(QDRANT_DEFAULT_DISTANCE, "Cosine");
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
}
