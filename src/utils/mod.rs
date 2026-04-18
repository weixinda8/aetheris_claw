pub mod async_utils;
pub mod concurrency;
pub mod crypto;
pub mod error;
pub mod validation;

pub use concurrency::{ConcurrencyGuard, ConcurrencyLimiter, ConcurrencyMetrics};
pub use error::{AetherisError, Result};
pub use validation::{ValidationError, ValidationResult};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_type_alias() {
        let success: Result<i32> = Ok(42);
        assert!(success.is_ok());
        assert_eq!(success.unwrap(), 42);

        let error: Result<i32> = Err(AetherisError::Internal("test error".to_string()));
        assert!(error.is_err());
    }

    #[test]
    fn test_aetheris_error_variants() {
        let io_error =
            AetherisError::Io(std::io::Error::new(std::io::ErrorKind::Other, "io error"));
        assert!(format!("{}", io_error).contains("IO error"));

        let config_error = AetherisError::Config("invalid config".to_string());
        assert!(format!("{}", config_error).contains("Configuration error"));

        let security_error = AetherisError::Security("access denied".to_string());
        assert!(format!("{}", security_error).contains("Security violation"));

        let not_found_error = AetherisError::NotFound("item not found".to_string());
        assert!(format!("{}", not_found_error).contains("Not found"));
    }
}
