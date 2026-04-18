use thiserror::Error;

pub type Result<T> = std::result::Result<T, AetherisError>;

#[derive(Error, Debug)]
pub enum AetherisError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("YAML serialization error: {0}")]
    YamlSerialization(#[from] serde_yaml::Error),

    #[error("Database error: {0}")]
    Database(#[from] Box<sqlx::Error>),

    #[error("HTTP error: {0}")]
    Http(#[from] Box<reqwest::Error>),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Task execution error: {0}")]
    TaskExecution(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Skill error: {0}")]
    Skill(String),

    #[error("SOUL error: {0}")]
    Soul(String),

    #[error("AgentSkills error: {0}")]
    AgentSkills(String),

    #[error("ClawHub error: {0}")]
    ClawHub(String),

    #[error("Security violation: {0}")]
    Security(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("Model error: {0}")]
    ModelError(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Intent validation error: {0}")]
    IntentValidation(String),

    #[error("Planning error: {0}")]
    Planning(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Qdrant error: {0}")]
    Qdrant(#[from] Box<qdrant_client::QdrantError>),

    #[error("Token budget exceeded: {0}")]
    TokenBudgetExceeded(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("CDN error: {0}")]
    Cdn(String),

    #[error("Bincode error: {0}")]
    Bincode(String),

    #[error("CLI config error: {0}")]
    CliConfig(String),

    #[error("Onboard error: {0}")]
    Onboard(String),

    #[error("External error: {0}")]
    External(String),

    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    #[error("Observability error: {0}")]
    Observability(String),
}

impl From<Box<bincode::ErrorKind>> for AetherisError {
    fn from(e: Box<bincode::ErrorKind>) -> Self {
        AetherisError::Bincode(e.to_string())
    }
}

impl From<crate::cli::config::CliConfigError> for AetherisError {
    fn from(e: crate::cli::config::CliConfigError) -> Self {
        AetherisError::CliConfig(e.to_string())
    }
}

impl From<crate::config::onboard::OnboardError> for AetherisError {
    fn from(e: crate::config::onboard::OnboardError) -> Self {
        AetherisError::Onboard(e.to_string())
    }
}

impl From<reqwest::Error> for AetherisError {
    fn from(e: reqwest::Error) -> Self {
        AetherisError::Http(Box::new(e))
    }
}

impl From<sqlx::Error> for AetherisError {
    fn from(e: sqlx::Error) -> Self {
        AetherisError::Database(Box::new(e))
    }
}

impl From<qdrant_client::QdrantError> for AetherisError {
    fn from(e: qdrant_client::QdrantError) -> Self {
        AetherisError::Qdrant(Box::new(e))
    }
}
