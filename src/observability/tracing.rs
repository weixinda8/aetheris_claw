use crate::utils::{AetherisError, Result};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Text,
    Json,
}

pub fn init_tracing_with_format(log_format: LogFormat) -> Result<()> {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,aetheris=debug"));

    match log_format {
        LogFormat::Text => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .try_init()
                .map_err(|e| AetherisError::Internal(e.to_string()))?;
        }
        LogFormat::Json => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .try_init()
                .map_err(|e| AetherisError::Internal(e.to_string()))?;
        }
    }

    Ok(())
}

pub fn init_tracing() -> Result<()> {
    init_tracing_with_format(LogFormat::Text)
}

pub fn init_structured_logging() -> Result<()> {
    init_tracing_with_format(LogFormat::Json)
}
