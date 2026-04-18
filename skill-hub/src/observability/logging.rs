use crate::utils::{Result, SkillHubError};
use tracing_subscriber::{
    fmt::{self, format::JsonFields, MakeWriter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Text,
    Json,
}

pub fn init_structured_logging(log_format: LogFormat) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,aetheris_skill_hub=debug,tower_http=debug".into());

    match log_format {
        LogFormat::Text => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer())
                .try_init()
                .map_err(|e| SkillHubError::Internal(format!("Failed to init text logging: {}", e)))?;
        }
        LogFormat::Json => {
            let json_layer = fmt::layer()
                .json()
                .with_target(true)
                .with_level(true)
                .with_file(true)
                .with_line_number(true)
                .with_current_span(true)
                .with_span_list(true);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(json_layer)
                .try_init()
                .map_err(|e| SkillHubError::Internal(format!("Failed to init JSON logging: {}", e)))?;
        }
    }

    Ok(())
}
