use crate::core::Task;
use crate::utils::validation::{ValidationError, ValidationResult};
use serde::{Deserialize, Serialize};
use chrono;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }
}

impl<T> ApiResponse<T> {
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Default for HealthResponse {
    fn default() -> Self {
        Self {
            status: "unknown".to_string(),
            version: "0.0.0".to_string(),
            timestamp: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitTaskRequest {
    pub description: String,
    pub priority: u8,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}

impl SubmitTaskRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.description, "description")?;
        validation::validate_string_length(&self.description, "description", 1, 2000)?;
        validation::validate_numeric_range(self.priority, "priority", 0u8, 10u8)?;

        if let Some(tags) = &self.tags {
            if tags.len() > 50 {
                return Err(ValidationError::InvalidField {
                    field: "tags".to_string(),
                    message: "must have at most 50 tags".to_string(),
                });
            }
            for tag in tags {
                if tag.len() > 50 {
                    return Err(ValidationError::InvalidField {
                        field: "tags".to_string(),
                        message: "each tag must be at most 50 characters".to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitTaskResponse {
    pub task_id: String,
    pub status: String,
    pub message: String,
}

impl Default for SubmitTaskResponse {
    fn default() -> Self {
        Self {
            task_id: String::new(),
            status: "unknown".to_string(),
            message: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub task: Task,
}

impl Default for TaskResponse {
    fn default() -> Self {
        Self {
            task: Task::new("default".to_string(), 0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct TaskListResponse {
    pub tasks: Vec<Task>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityValidationRequest {
    pub task: Task,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct SecurityValidationResponse {
    pub task_id: String,
    pub passed: bool,
    pub violations: Vec<String>,
    pub warnings: Vec<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AgentListResponse {
    pub agents: Vec<crate::agent::AgentConfig>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AgentResponse {
    pub config: crate::agent::AgentConfig,
    pub state: crate::agent::AgentState,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AuditEventListResponse {
    pub events: Vec<crate::security::audit::AuditEvent>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub total_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub active_agents: usize,
    pub uptime_seconds: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Default for MetricsResponse {
    fn default() -> Self {
        Self {
            total_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            active_agents: 0,
            uptime_seconds: 0,
            timestamp: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub message_type: String,
    pub task_id: Option<String>,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl WebSocketMessage {
    pub fn new(message_type: String, task_id: Option<String>, data: serde_json::Value) -> Self {
        Self {
            message_type,
            task_id,
            data,
            timestamp: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsResponse {
    pub metrics: crate::observability::SystemMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetricsListResponse {
    pub tasks: Vec<crate::observability::TaskMetrics>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertListResponse {
    pub alerts: Vec<crate::observability::Alert>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveAlertRequest {
    pub alert_id: String,
}

impl ResolveAlertRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.alert_id, "alert_id")?;
        validation::validate_string_length(&self.alert_id, "alert_id", 1, 100)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAlertRequest {
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub task_id: Option<String>,
    pub agent_id: Option<String>,
}

impl CreateAlertRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.alert_type, "alert_type")?;
        validation::validate_string_length(&self.alert_type, "alert_type", 1, 100)?;

        validation::validate_not_empty(&self.severity, "severity")?;
        validation::validate_string_length(&self.severity, "severity", 1, 50)?;

        validation::validate_not_empty(&self.message, "message")?;
        validation::validate_string_length(&self.message, "message", 1, 5000)?;

        if let Some(task_id) = &self.task_id {
            validation::validate_string_length(task_id, "task_id", 1, 100)?;
        }

        if let Some(agent_id) = &self.agent_id {
            validation::validate_string_length(agent_id, "agent_id", 1, 100)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PipelineStatus {
    Stopped,
    Running,
    Paused,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: PipelineStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePipelineRequest {
    pub name: String,
    pub description: Option<String>,
    pub config: serde_json::Value,
}

impl CreatePipelineRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.name, "name")?;
        validation::validate_string_length(&self.name, "name", 1, 200)?;

        if let Some(desc) = &self.description {
            if !desc.is_empty() {
                validation::validate_string_length(desc, "description", 1, 5000)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePipelineRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub config: Option<serde_json::Value>,
}

impl UpdatePipelineRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        if let Some(name) = &self.name {
            validation::validate_not_empty(name, "name")?;
            validation::validate_string_length(name, "name", 1, 200)?;
        }

        if let Some(desc) = &self.description {
            if !desc.is_empty() {
                validation::validate_string_length(desc, "description", 1, 5000)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResponse {
    pub pipeline: Pipeline,
}

impl Default for PipelineResponse {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            pipeline: Pipeline {
                id: String::new(),
                name: String::new(),
                description: None,
                status: PipelineStatus::Stopped,
                created_at: now,
                updated_at: now,
                started_at: None,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct PipelineListResponse {
    pub pipelines: Vec<Pipeline>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineMetrics {
    pub pipeline_id: String,
    pub records_processed: u64,
    pub errors: u64,
    pub throughput_per_second: f64,
    pub latency_ms: f64,
    pub uptime_seconds: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineLogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct PipelineLogsResponse {
    pub logs: Vec<PipelineLogEntry>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: crate::api::auth::UserRole,
}

impl CreateUserRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.username, "username")?;
        validation::validate_string_length(&self.username, "username", 3, 50)?;

        validation::validate_not_empty(&self.password, "password")?;
        validation::validate_string_length(&self.password, "password", 8, 100)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRoleRequest {
    pub role: crate::api::auth::UserRole,
}

impl UpdateUserRoleRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub role: crate::api::auth::UserRole,
}

impl Default for UserResponse {
    fn default() -> Self {
        Self {
            user_id: uuid::Uuid::nil(),
            username: String::new(),
            role: crate::api::auth::UserRole::Viewer,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct UserListResponse {
    pub users: Vec<UserResponse>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAlertRuleRequest {
    pub name: String,
    pub description: Option<String>,
    pub rule_type: crate::observability::AlertRuleType,
    pub condition: crate::observability::AlertCondition,
    pub severity: crate::observability::AlertSeverity,
    pub channel_ids: Vec<String>,
    pub escalation_policy_id: Option<String>,
    pub evaluation_interval_seconds: u64,
}

impl CreateAlertRuleRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.name, "name")?;
        validation::validate_string_length(&self.name, "name", 1, 200)?;

        if let Some(desc) = &self.description {
            if !desc.is_empty() {
                validation::validate_string_length(desc, "description", 1, 5000)?;
            }
        }

        if self.channel_ids.is_empty() {
            return Err(ValidationError::InvalidField {
                field: "channel_ids".to_string(),
                message: "must have at least one channel ID".to_string(),
            });
        }

        if self.channel_ids.len() > 50 {
            return Err(ValidationError::InvalidField {
                field: "channel_ids".to_string(),
                message: "must have at most 50 channel IDs".to_string(),
            });
        }

        for channel_id in &self.channel_ids {
            validation::validate_string_length(channel_id, "channel_id", 1, 100)?;
        }

        if let Some(policy_id) = &self.escalation_policy_id {
            validation::validate_string_length(policy_id, "escalation_policy_id", 1, 100)?;
        }

        validation::validate_numeric_range(
            self.evaluation_interval_seconds,
            "evaluation_interval_seconds",
            1u64,
            86400u64,
        )?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAlertRuleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub condition: Option<crate::observability::AlertCondition>,
    pub severity: Option<crate::observability::AlertSeverity>,
    pub channel_ids: Option<Vec<String>>,
    pub escalation_policy_id: Option<String>,
    pub evaluation_interval_seconds: Option<u64>,
    pub status: Option<crate::observability::AlertRuleStatus>,
}

impl UpdateAlertRuleRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        if let Some(name) = &self.name {
            validation::validate_not_empty(name, "name")?;
            validation::validate_string_length(name, "name", 1, 200)?;
        }

        if let Some(desc) = &self.description {
            if !desc.is_empty() {
                validation::validate_string_length(desc, "description", 1, 5000)?;
            }
        }

        if let Some(channel_ids) = &self.channel_ids {
            if channel_ids.is_empty() {
                return Err(ValidationError::InvalidField {
                    field: "channel_ids".to_string(),
                    message: "must have at least one channel ID".to_string(),
                });
            }

            if channel_ids.len() > 50 {
                return Err(ValidationError::InvalidField {
                    field: "channel_ids".to_string(),
                    message: "must have at most 50 channel IDs".to_string(),
                });
            }

            for channel_id in channel_ids {
                validation::validate_string_length(channel_id, "channel_id", 1, 100)?;
            }
        }

        if let Some(policy_id) = &self.escalation_policy_id {
            validation::validate_string_length(policy_id, "escalation_policy_id", 1, 100)?;
        }

        if let Some(interval) = self.evaluation_interval_seconds {
            validation::validate_numeric_range(
                interval,
                "evaluation_interval_seconds",
                1u64,
                86400u64,
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRuleListResponse {
    pub rules: Vec<crate::observability::AlertRule>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertHistoryListResponse {
    pub history: Vec<crate::observability::AlertHistory>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuteAlertRuleRequest {
    pub reason: String,
    pub muted_by: String,
    pub duration_seconds: Option<u64>,
}

impl MuteAlertRuleRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.reason, "reason")?;
        validation::validate_string_length(&self.reason, "reason", 1, 1000)?;

        validation::validate_not_empty(&self.muted_by, "muted_by")?;
        validation::validate_string_length(&self.muted_by, "muted_by", 1, 100)?;

        if let Some(duration) = self.duration_seconds {
            validation::validate_numeric_range(duration, "duration_seconds", 1u64, 31536000u64)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcknowledgeAlertRequest {
    pub acknowledged_by: String,
}

impl AcknowledgeAlertRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.acknowledged_by, "acknowledged_by")?;
        validation::validate_string_length(&self.acknowledged_by, "acknowledged_by", 1, 100)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNotificationChannelRequest {
    pub name: String,
    pub channel_type: crate::observability::NotificationChannelType,
    pub config: crate::observability::NotificationChannelConfig,
}

impl CreateNotificationChannelRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.name, "name")?;
        validation::validate_string_length(&self.name, "name", 1, 200)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannelListResponse {
    pub channels: Vec<crate::observability::NotificationChannel>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEscalationPolicyRequest {
    pub name: String,
    pub steps: Vec<crate::observability::EscalationStep>,
}

impl CreateEscalationPolicyRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.name, "name")?;
        validation::validate_string_length(&self.name, "name", 1, 200)?;

        if self.steps.is_empty() {
            return Err(ValidationError::InvalidField {
                field: "steps".to_string(),
                message: "must have at least one escalation step".to_string(),
            });
        }

        if self.steps.len() > 20 {
            return Err(ValidationError::InvalidField {
                field: "steps".to_string(),
                message: "must have at most 20 escalation steps".to_string(),
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPolicyListResponse {
    pub policies: Vec<crate::observability::EscalationPolicy>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterModelRequest {
    pub name: String,
    pub format: crate::ai::inference::ModelFormat,
    pub version: String,
    pub path: String,
    pub description: Option<String>,
}

impl RegisterModelRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.name, "name")?;
        validation::validate_string_length(&self.name, "name", 1, 200)?;

        validation::validate_not_empty(&self.version, "version")?;
        validation::validate_string_length(&self.version, "version", 1, 50)?;

        validation::validate_not_empty(&self.path, "path")?;
        validation::validate_string_length(&self.path, "path", 1, 500)?;

        if let Some(desc) = &self.description {
            if !desc.is_empty() {
                validation::validate_string_length(desc, "description", 1, 5000)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub model: crate::ai::inference::Model,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateModelRequest {
    pub name: Option<String>,
    pub format: Option<crate::ai::inference::ModelFormat>,
    pub version: Option<String>,
    pub path: Option<String>,
    pub description: Option<String>,
}

impl UpdateModelRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        if let Some(name) = &self.name {
            validation::validate_not_empty(name, "name")?;
            validation::validate_string_length(name, "name", 1, 200)?;
        }

        if let Some(version) = &self.version {
            validation::validate_not_empty(version, "version")?;
            validation::validate_string_length(version, "version", 1, 50)?;
        }

        if let Some(path) = &self.path {
            validation::validate_not_empty(path, "path")?;
            validation::validate_string_length(path, "path", 1, 500)?;
        }

        if let Some(desc) = &self.description {
            if !desc.is_empty() {
                validation::validate_string_length(desc, "description", 1, 5000)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelListResponse {
    pub models: Vec<crate::ai::inference::Model>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub model_id: String,
    pub data: serde_json::Value,
    pub parameters: Option<std::collections::HashMap<String, serde_json::Value>>,
    pub use_cache: Option<bool>,
    pub cache_ttl: Option<u64>,
}

impl InferenceRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.model_id, "model_id")?;
        validation::validate_string_length(&self.model_id, "model_id", 1, 100)?;

        if let Some(ttl) = self.cache_ttl {
            validation::validate_numeric_range(ttl, "cache_ttl", 0u64, 86400u64)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub output: crate::ai::inference::InferenceOutput,
    pub from_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceMetricsResponse {
    pub metrics: crate::ai::inference::InferenceMetricsData,
    pub cache_stats: Option<crate::ai::inference::CacheStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectAnomalyRequest {
    pub features: std::collections::HashMap<String, f64>,
    pub method: Option<crate::ai::anomaly_detection::AnomalyDetectionMethod>,
}

impl DetectAnomalyRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        if self.features.is_empty() {
            return Err(ValidationError::InvalidField {
                field: "features".to_string(),
                message: "must have at least one feature".to_string(),
            });
        }

        if self.features.len() > 100 {
            return Err(ValidationError::InvalidField {
                field: "features".to_string(),
                message: "must have at most 100 features".to_string(),
            });
        }

        for key in self.features.keys() {
            validation::validate_string_length(key, "feature_name", 1, 100)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectAnomalyResponse {
    pub anomaly: crate::ai::anomaly_detection::Anomaly,
}

impl Default for DetectAnomalyResponse {
    fn default() -> Self {
        Self {
            anomaly: crate::ai::anomaly_detection::Anomaly::new(
                0.0,
                false,
                std::collections::HashMap::new(),
                crate::ai::anomaly_detection::AnomalyDetectionMethod::Statistical3Sigma,
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AnomalyListResponse {
    pub anomalies: Vec<crate::ai::anomaly_detection::Anomaly>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDetectAnomalyRequest {
    pub data: Vec<std::collections::HashMap<String, f64>>,
    pub method: Option<crate::ai::anomaly_detection::AnomalyDetectionMethod>,
}

impl BatchDetectAnomalyRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        if self.data.is_empty() {
            return Err(ValidationError::InvalidField {
                field: "data".to_string(),
                message: "must have at least one data point".to_string(),
            });
        }

        if self.data.len() > 10000 {
            return Err(ValidationError::InvalidField {
                field: "data".to_string(),
                message: "must have at most 10000 data points".to_string(),
            });
        }

        for (i, features) in self.data.iter().enumerate() {
            if features.is_empty() {
                return Err(ValidationError::InvalidField {
                    field: format!("data[{}]", i),
                    message: "each data point must have at least one feature".to_string(),
                });
            }

            if features.len() > 100 {
                return Err(ValidationError::InvalidField {
                    field: format!("data[{}]", i),
                    message: "each data point must have at most 100 features".to_string(),
                });
            }

            for key in features.keys() {
                validation::validate_string_length(key, &format!("data[{}].{}", i, key), 1, 100)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDetectAnomalyResponse {
    pub anomalies: Vec<crate::ai::anomaly_detection::Anomaly>,
    pub total_anomalies: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AnomalyVisualizationResponse {
    pub visualization_data: crate::ai::anomaly_detection::AnomalyVisualizationData,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitModelRequest {
    pub data: Vec<std::collections::HashMap<String, f64>>,
    pub method: crate::ai::anomaly_detection::AnomalyDetectionMethod,
}

impl FitModelRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        if self.data.is_empty() {
            return Err(ValidationError::InvalidField {
                field: "data".to_string(),
                message: "must have at least one data point for fitting".to_string(),
            });
        }

        if self.data.len() > 100000 {
            return Err(ValidationError::InvalidField {
                field: "data".to_string(),
                message: "must have at most 100000 data points for fitting".to_string(),
            });
        }

        for (i, features) in self.data.iter().enumerate() {
            if features.is_empty() {
                return Err(ValidationError::InvalidField {
                    field: format!("data[{}]", i),
                    message: "each data point must have at least one feature".to_string(),
                });
            }

            if features.len() > 100 {
                return Err(ValidationError::InvalidField {
                    field: format!("data[{}]", i),
                    message: "each data point must have at most 100 features".to_string(),
                });
            }

            for key in features.keys() {
                validation::validate_string_length(key, &format!("data[{}].{}", i, key), 1, 100)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastRequest {
    pub timestamps: Vec<chrono::DateTime<chrono::Utc>>,
    pub values: Vec<f64>,
    pub method: Option<crate::ai::forecasting::ForecastingMethod>,
    pub horizon: usize,
    pub with_confidence: Option<bool>,
}

impl ForecastRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        if self.timestamps.is_empty() {
            return Err(ValidationError::InvalidField {
                field: "timestamps".to_string(),
                message: "must have at least one timestamp".to_string(),
            });
        }

        if self.timestamps.len() > 100000 {
            return Err(ValidationError::InvalidField {
                field: "timestamps".to_string(),
                message: "must have at most 100000 timestamps".to_string(),
            });
        }

        if self.values.is_empty() {
            return Err(ValidationError::InvalidField {
                field: "values".to_string(),
                message: "must have at least one value".to_string(),
            });
        }

        if self.values.len() > 100000 {
            return Err(ValidationError::InvalidField {
                field: "values".to_string(),
                message: "must have at most 100000 values".to_string(),
            });
        }

        if self.timestamps.len() != self.values.len() {
            return Err(ValidationError::InvalidField {
                field: "timestamps, values".to_string(),
                message: "timestamps and values must have the same length".to_string(),
            });
        }

        validation::validate_numeric_range(self.horizon, "horizon", 1usize, 1000usize)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastResponse {
    pub forecast: crate::ai::forecasting::Forecast,
}

impl Default for ForecastResponse {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            forecast: crate::ai::forecasting::Forecast::new(
                vec![now],
                vec![0.0],
                crate::ai::forecasting::ForecastingMethod::ARIMA,
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiStepForecastRequest {
    pub timestamps: Vec<chrono::DateTime<chrono::Utc>>,
    pub values: Vec<f64>,
    pub method: Option<crate::ai::forecasting::ForecastingMethod>,
    pub horizon: usize,
    pub strategy: crate::ai::forecasting::multi_step::MultiStepStrategy,
}

impl MultiStepForecastRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        if self.timestamps.is_empty() {
            return Err(ValidationError::InvalidField {
                field: "timestamps".to_string(),
                message: "must have at least one timestamp".to_string(),
            });
        }

        if self.timestamps.len() > 100000 {
            return Err(ValidationError::InvalidField {
                field: "timestamps".to_string(),
                message: "must have at most 100000 timestamps".to_string(),
            });
        }

        if self.values.is_empty() {
            return Err(ValidationError::InvalidField {
                field: "values".to_string(),
                message: "must have at least one value".to_string(),
            });
        }

        if self.values.len() > 100000 {
            return Err(ValidationError::InvalidField {
                field: "values".to_string(),
                message: "must have at most 100000 values".to_string(),
            });
        }

        if self.timestamps.len() != self.values.len() {
            return Err(ValidationError::InvalidField {
                field: "timestamps, values".to_string(),
                message: "timestamps and values must have the same length".to_string(),
            });
        }

        validation::validate_numeric_range(self.horizon, "horizon", 1usize, 1000usize)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelectionRequest {
    pub timestamps: Vec<chrono::DateTime<chrono::Utc>>,
    pub values: Vec<f64>,
    pub criteria: crate::ai::forecasting::ModelSelectionCriteria,
    pub horizon: usize,
    pub candidate_methods: Option<Vec<crate::ai::forecasting::ForecastingMethod>>,
}

impl ModelSelectionRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        if self.timestamps.is_empty() {
            return Err(ValidationError::InvalidField {
                field: "timestamps".to_string(),
                message: "must have at least one timestamp".to_string(),
            });
        }

        if self.timestamps.len() > 100000 {
            return Err(ValidationError::InvalidField {
                field: "timestamps".to_string(),
                message: "must have at most 100000 timestamps".to_string(),
            });
        }

        if self.values.is_empty() {
            return Err(ValidationError::InvalidField {
                field: "values".to_string(),
                message: "must have at least one value".to_string(),
            });
        }

        if self.values.len() > 100000 {
            return Err(ValidationError::InvalidField {
                field: "values".to_string(),
                message: "must have at most 100000 values".to_string(),
            });
        }

        if self.timestamps.len() != self.values.len() {
            return Err(ValidationError::InvalidField {
                field: "timestamps, values".to_string(),
                message: "timestamps and values must have the same length".to_string(),
            });
        }

        validation::validate_numeric_range(self.horizon, "horizon", 1usize, 1000usize)?;

        if let Some(candidates) = &self.candidate_methods {
            if candidates.is_empty() {
                return Err(ValidationError::InvalidField {
                    field: "candidate_methods".to_string(),
                    message: "must have at least one candidate method if provided".to_string(),
                });
            }

            if candidates.len() > 20 {
                return Err(ValidationError::InvalidField {
                    field: "candidate_methods".to_string(),
                    message: "must have at most 20 candidate methods".to_string(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelectionResponse {
    pub best_method: crate::ai::forecasting::ForecastingMethod,
    pub best_model_name: String,
    pub forecast: crate::ai::forecasting::Forecast,
    pub performance_history: Vec<crate::ai::forecasting::auto_select::ModelPerformance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ForecastHistoryResponse {
    pub forecasts: Vec<crate::ai::forecasting::Forecast>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddEntityRequest {
    pub entity_type: crate::ai::knowledge_graph::EntityType,
    pub name: String,
    pub description: Option<String>,
    pub properties: Option<std::collections::HashMap<String, serde_json::Value>>,
}

impl AddEntityRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.name, "name")?;
        validation::validate_string_length(&self.name, "name", 1, 200)?;

        if let Some(desc) = &self.description {
            if !desc.is_empty() {
                validation::validate_string_length(desc, "description", 1, 5000)?;
            }
        }

        if let Some(properties) = &self.properties {
            if properties.len() > 100 {
                return Err(ValidationError::InvalidField {
                    field: "properties".to_string(),
                    message: "must have at most 100 properties".to_string(),
                });
            }

            for key in properties.keys() {
                validation::validate_string_length(key, "property_name", 1, 100)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AddEntityResponse {
    pub entity_id: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddRelationshipRequest {
    pub relationship_type: crate::ai::knowledge_graph::RelationshipType,
    pub source_id: String,
    pub target_id: String,
    pub properties: Option<std::collections::HashMap<String, serde_json::Value>>,
}

impl AddRelationshipRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.source_id, "source_id")?;
        validation::validate_string_length(&self.source_id, "source_id", 1, 100)?;

        validation::validate_not_empty(&self.target_id, "target_id")?;
        validation::validate_string_length(&self.target_id, "target_id", 1, 100)?;

        if let Some(properties) = &self.properties {
            if properties.len() > 100 {
                return Err(ValidationError::InvalidField {
                    field: "properties".to_string(),
                    message: "must have at most 100 properties".to_string(),
                });
            }

            for key in properties.keys() {
                validation::validate_string_length(key, "property_name", 1, 100)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AddRelationshipResponse {
    pub relationship_id: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub keywords: Option<Vec<String>>,
    pub entity_type: Option<crate::ai::knowledge_graph::EntityType>,
    pub relationship_type: Option<crate::ai::knowledge_graph::RelationshipType>,
    pub max_results: Option<usize>,
}

impl SearchRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        if let Some(keywords) = &self.keywords {
            if keywords.is_empty() {
                return Err(ValidationError::InvalidField {
                    field: "keywords".to_string(),
                    message: "must have at least one keyword if provided".to_string(),
                });
            }

            if keywords.len() > 50 {
                return Err(ValidationError::InvalidField {
                    field: "keywords".to_string(),
                    message: "must have at most 50 keywords".to_string(),
                });
            }

            for keyword in keywords {
                validation::validate_string_length(keyword, "keyword", 1, 200)?;
            }
        }

        if let Some(max_results) = self.max_results {
            validation::validate_numeric_range(max_results, "max_results", 1usize, 1000usize)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct SearchResponse {
    pub entities: Vec<crate::ai::knowledge_graph::Entity>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseQueryRequest {
    pub tags: Option<Vec<String>>,
    pub search_text: Option<String>,
    pub limit: Option<usize>,
}

impl CaseQueryRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        if let Some(tags) = &self.tags {
            if tags.len() > 50 {
                return Err(ValidationError::InvalidField {
                    field: "tags".to_string(),
                    message: "must have at most 50 tags".to_string(),
                });
            }

            for tag in tags {
                validation::validate_string_length(tag, "tag", 1, 100)?;
            }
        }

        if let Some(search_text) = &self.search_text {
            if !search_text.is_empty() {
                validation::validate_string_length(search_text, "search_text", 1, 5000)?;
            }
        }

        if let Some(limit) = self.limit {
            validation::validate_numeric_range(limit, "limit", 1usize, 1000usize)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct CaseListResponse {
    pub cases: Vec<crate::ai::knowledge_graph::MaintenanceCase>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddCaseRequest {
    pub title: String,
    pub description: String,
    pub device_ids: Option<Vec<String>>,
    pub fault_ids: Option<Vec<String>>,
    pub solution_ids: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub resolution_summary: Option<String>,
    pub root_cause: Option<String>,
    pub duration_minutes: Option<u32>,
}

impl AddCaseRequest {
    pub fn validate(&self) -> ValidationResult<()> {
        use crate::utils::validation;

        validation::validate_not_empty(&self.title, "title")?;
        validation::validate_string_length(&self.title, "title", 1, 500)?;

        validation::validate_not_empty(&self.description, "description")?;
        validation::validate_string_length(&self.description, "description", 1, 50000)?;

        if let Some(device_ids) = &self.device_ids {
            if device_ids.len() > 100 {
                return Err(ValidationError::InvalidField {
                    field: "device_ids".to_string(),
                    message: "must have at most 100 device IDs".to_string(),
                });
            }

            for device_id in device_ids {
                validation::validate_string_length(device_id, "device_id", 1, 100)?;
            }
        }

        if let Some(fault_ids) = &self.fault_ids {
            if fault_ids.len() > 100 {
                return Err(ValidationError::InvalidField {
                    field: "fault_ids".to_string(),
                    message: "must have at most 100 fault IDs".to_string(),
                });
            }

            for fault_id in fault_ids {
                validation::validate_string_length(fault_id, "fault_id", 1, 100)?;
            }
        }

        if let Some(solution_ids) = &self.solution_ids {
            if solution_ids.len() > 100 {
                return Err(ValidationError::InvalidField {
                    field: "solution_ids".to_string(),
                    message: "must have at most 100 solution IDs".to_string(),
                });
            }

            for solution_id in solution_ids {
                validation::validate_string_length(solution_id, "solution_id", 1, 100)?;
            }
        }

        if let Some(tags) = &self.tags {
            if tags.len() > 50 {
                return Err(ValidationError::InvalidField {
                    field: "tags".to_string(),
                    message: "must have at most 50 tags".to_string(),
                });
            }

            for tag in tags {
                validation::validate_string_length(tag, "tag", 1, 100)?;
            }
        }

        if let Some(resolution_summary) = &self.resolution_summary {
            if !resolution_summary.is_empty() {
                validation::validate_string_length(
                    resolution_summary,
                    "resolution_summary",
                    1,
                    10000,
                )?;
            }
        }

        if let Some(root_cause) = &self.root_cause {
            if !root_cause.is_empty() {
                validation::validate_string_length(root_cause, "root_cause", 1, 5000)?;
            }
        }

        if let Some(duration) = self.duration_minutes {
            validation::validate_numeric_range(duration, "duration_minutes", 0u32, 525600u32)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AddCaseResponse {
    pub case_id: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphVisualizationResponse {
    pub visualization: serde_json::Value,
}

impl Default for GraphVisualizationResponse {
    fn default() -> Self {
        Self {
            visualization: serde_json::Value::Null,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitFeedbackRequest {
    pub model_id: String,
    pub prediction_id: String,
    pub feedback_type: String,
    pub comment: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct SubmitFeedbackResponse {
    pub feedback_id: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct FeedbackListResponse {
    pub feedbacks: Vec<crate::ai::adaptive_learning::Feedback>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateModelVersionRequest {
    pub model_id: String,
    pub version: String,
    pub description: Option<String>,
    pub checksum: Option<String>,
    pub path: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct CreateModelVersionResponse {
    pub version_id: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ModelVersionListResponse {
    pub versions: Vec<crate::ai::adaptive_learning::ModelVersion>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackModelRequest {
    pub version_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackModelResponse {
    pub rollback_event: crate::ai::adaptive_learning::RollbackEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartABTestRequest {
    pub name: String,
    pub description: Option<String>,
    pub model_id: String,
    pub version_a: String,
    pub version_b: String,
    pub traffic_split: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct StartABTestResponse {
    pub test_id: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ABTestListResponse {
    pub tests: Vec<crate::ai::adaptive_learning::ABTest>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestResultResponse {
    pub result: crate::ai::adaptive_learning::ABTestResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub message_type: String,
    pub sender_id: String,
    pub receiver_id: Option<String>,
    pub topic: Option<String>,
    pub payload: serde_json::Value,
    pub priority: Option<u8>,
    pub ttl: Option<u64>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct MessageResponse {
    pub message_id: String,
    pub success: bool,
    pub message: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeTopicRequest {
    pub agent_id: String,
    pub topic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeTopicRequest {
    pub agent_id: String,
    pub topic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct MessageListResponse {
    pub messages: Vec<crate::agent::communication::Message>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterAgentRequest {
    pub agent_id: String,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct TopicListResponse {
    pub topics: Vec<String>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct TopicSubscribersResponse {
    pub topic: String,
    pub subscribers: Vec<String>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposeTaskRequest {
    pub task_description: String,
    pub options: Option<crate::agent::task_decomposer::DecompositionOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposeTaskResponse {
    pub decomposed_task: crate::agent::task_decomposer::DecomposedTask,
    pub validation: Option<crate::agent::task_decomposer::DecompositionValidationResult>,
}

impl Default for DecomposeTaskResponse {
    fn default() -> Self {
        Self {
            decomposed_task: crate::agent::task_decomposer::DecomposedTask {
                original_task_id: String::new(),
                original_description: String::new(),
                sub_tasks: Vec::new(),
                requires_human_review: false,
                review_notes: None,
                created_at: chrono::Utc::now(),
                confidence: 0.0,
            },
            validation: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchAgentRequest {
    pub requirement: crate::agent::matcher::TaskRequirement,
    pub agents: Vec<crate::agent::matcher::AgentProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchAgentResponse {
    pub result: crate::agent::matcher::MatchResult,
}

impl Default for MatchAgentResponse {
    fn default() -> Self {
        Self {
            result: crate::agent::matcher::MatchResult {
                task_requirement: crate::agent::matcher::TaskRequirement::default(),
                matches: Vec::new(),
                best_match: None,
                fallback_agents: Vec::new(),
                created_at: chrono::Utc::now(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordLineageRequest {
    pub nodes: Vec<crate::data_governance::LineageNode>,
    pub edges: Vec<crate::data_governance::LineageEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordLineageResponse {
    pub node_ids: Vec<String>,
    pub edge_ids: Vec<String>,
    pub success: bool,
}

impl Default for RecordLineageResponse {
    fn default() -> Self {
        Self {
            node_ids: Vec::new(),
            edge_ids: Vec::new(),
            success: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct LineageResponse {
    pub lineage: crate::data_governance::DataLineage,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageQueryRequest {
    pub node_id: String,
    pub depth: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct LineageQueryResponse {
    pub nodes: Vec<crate::data_governance::LineageNode>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageGraphResponse {
    pub graph_data: serde_json::Value,
}

impl Default for LineageGraphResponse {
    fn default() -> Self {
        Self {
            graph_data: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysisRequest {
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ImpactAnalysisResponse {
    pub affected_nodes: usize,
    pub node_type_counts: std::collections::HashMap<String, usize>,
    pub affected_nodes_list: Vec<crate::data_governance::LineageNode>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyDataRequest {
    pub data: String,
    pub metadata: Option<std::collections::HashMap<String, String>>,
    pub strategy_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyDataResponse {
    pub result: crate::data_governance::ClassificationResult,
}

impl Default for ClassifyDataResponse {
    fn default() -> Self {
        Self {
            result: crate::data_governance::ClassificationResult {
                data_id: String::new(),
                classification: crate::data_governance::DataClassification::Internal,
                confidence: 0.0,
                tags: Vec::new(),
                classified_by: String::new(),
                classified_at: chrono::Utc::now(),
                needs_review: false,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskDataRequest {
    pub data: String,
    pub rule_id: Option<uuid::Uuid>,
    pub user_id: Option<String>,
    pub is_static: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskDataResponse {
    pub result: crate::data_governance::MaskingResult,
}

impl Default for MaskDataResponse {
    fn default() -> Self {
        Self {
            result: crate::data_governance::MaskingResult {
                original_data: String::new(),
                masked_data: String::new(),
                rule_id: None,
                algorithm: crate::data_governance::MaskingAlgorithm::Mask,
                masked_at: chrono::Utc::now(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateComplianceReportRequest {
    pub name: String,
    pub description: String,
    pub standard: crate::data_governance::ComplianceStandard,
    pub format: crate::data_governance::ReportFormat,
    pub template_id: Option<uuid::Uuid>,
    pub generated_by: String,
    pub period_start: Option<chrono::DateTime<chrono::Utc>>,
    pub period_end: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateComplianceReportResponse {
    pub report: crate::data_governance::ComplianceReport,
}

impl Default for GenerateComplianceReportResponse {
    fn default() -> Self {
        Self {
            report: crate::data_governance::ComplianceReport::new(
                String::new(),
                String::new(),
                crate::data_governance::ComplianceStandard::GDPR,
                crate::data_governance::ReportFormat::PDF,
                crate::data_governance::compliance_reporting::ReportType::ComplianceCheck,
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ListComplianceReportsResponse {
    pub reports: Vec<crate::data_governance::ComplianceReport>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignComplianceReportRequest {
    pub report_id: uuid::Uuid,
    pub signer: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignComplianceReportResponse {
    pub report: crate::data_governance::ComplianceReport,
}

impl Default for SignComplianceReportResponse {
    fn default() -> Self {
        Self {
            report: crate::data_governance::ComplianceReport::new(
                String::new(),
                String::new(),
                crate::data_governance::ComplianceStandard::GDPR,
                crate::data_governance::ReportFormat::PDF,
                crate::data_governance::compliance_reporting::ReportType::ComplianceCheck,
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportComplianceReportRequest {
    pub report_id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportComplianceReportResponse {
    pub format: crate::data_governance::ReportFormat,
    pub content: String,
}

impl Default for ExportComplianceReportResponse {
    fn default() -> Self {
        Self {
            format: crate::data_governance::ReportFormat::PDF,
            content: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterDataRequest {
    pub data: Vec<crate::edge::EdgeData>,
    pub config_key: Option<String>,
    pub stream_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterDataResponse {
    pub filtered_data: Vec<crate::edge::EdgeData>,
    pub original_count: usize,
    pub filtered_count: usize,
    pub processing_time_ms: f64,
    pub compression_ratio: Option<f64>,
}

impl Default for FilterDataResponse {
    fn default() -> Self {
        Self {
            filtered_data: Vec::new(),
            original_count: 0,
            filtered_count: 0,
            processing_time_ms: 0.0,
            compression_ratio: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterStatisticsResponse {
    pub total_records: u64,
    pub filtered_records: u64,
    pub uptime_seconds: i64,
    pub average_processing_time_ms: f64,
    pub average_compression_ratio: Option<f64>,
    pub last_record_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for FilterStatisticsResponse {
    fn default() -> Self {
        Self {
            total_records: 0,
            filtered_records: 0,
            uptime_seconds: 0,
            average_processing_time_ms: 0.0,
            average_compression_ratio: None,
            last_record_time: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFilterConfigRequest {
    pub config: crate::edge::FilterConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct UpdateFilterConfigResponse {
    pub success: bool,
    pub config_key: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct GetFilterConfigResponse {
    pub config_key: String,
    pub config: crate::edge::FilterConfig,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct FilterConfigListResponse {
    pub configs: Vec<(String, crate::edge::FilterConfig)>,
    pub total: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportLineageRequest {
    pub format: String,
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportLineageResponse {
    pub format: String,
    pub content: String,
    pub filename: String,
}

impl Default for ExportLineageResponse {
    fn default() -> Self {
        Self {
            format: "json".to_string(),
            content: String::new(),
            filename: "lineage_export.json".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistLineageRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct PersistLineageResponse {
    pub success: bool,
    pub path: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadLineageRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct LoadLineageResponse {
    pub success: bool,
    pub loaded: bool,
}

