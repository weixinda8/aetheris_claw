use serde::{Deserialize, Serialize};
use thiserror::Error;
use validator::Validate;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Invalid field: {field} - {message}")]
    InvalidField { field: String, message: String },

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid value for field {field}: {value}")]
    InvalidValue { field: String, value: String },
}

pub type ValidationResult<T> = Result<T, ValidationError>;

pub fn validate<T: Validate>(value: &T) -> ValidationResult<()> {
    value
        .validate()
        .map_err(|e| ValidationError::Validation(format!("{:?}", e)))
}

pub fn validate_not_empty(value: &str, field_name: &str) -> ValidationResult<()> {
    if value.trim().is_empty() {
        Err(ValidationError::MissingField(field_name.to_string()))
    } else {
        Ok(())
    }
}

pub fn validate_string_length(
    value: &str,
    field_name: &str,
    min: usize,
    max: usize,
) -> ValidationResult<()> {
    validate_not_empty(value, field_name)?;

    let len = value.trim().len();
    if len < min {
        Err(ValidationError::InvalidField {
            field: field_name.to_string(),
            message: format!("must be at least {} characters", min),
        })
    } else if len > max {
        Err(ValidationError::InvalidField {
            field: field_name.to_string(),
            message: format!("must be at most {} characters", max),
        })
    } else {
        Ok(())
    }
}

pub fn validate_numeric_range<T: PartialOrd + std::fmt::Display>(
    value: T,
    field_name: &str,
    min: T,
    max: T,
) -> ValidationResult<()> {
    if value < min || value > max {
        Err(ValidationError::InvalidValue {
            field: field_name.to_string(),
            value: format!("{}", value),
        })
    } else {
        Ok(())
    }
}

pub fn validate_uuid(value: &str, field_name: &str) -> ValidationResult<()> {
    validate_not_empty(value, field_name)?;

    if uuid::Uuid::parse_str(value).is_err() {
        Err(ValidationError::InvalidField {
            field: field_name.to_string(),
            message: "must be a valid UUID".to_string(),
        })
    } else {
        Ok(())
    }
}
