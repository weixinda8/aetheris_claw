pub mod ab_testing;
pub mod feedback;
pub mod monitoring;
pub mod online;
pub mod rollback;
pub mod versioning;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FeedbackType {
    Positive,
    Negative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub id: String,
    pub model_id: String,
    pub prediction_id: String,
    pub feedback_type: FeedbackType,
    pub comment: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Feedback {
    pub fn new(
        model_id: String,
        prediction_id: String,
        feedback_type: FeedbackType,
        comment: Option<String>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        created_by: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            model_id,
            prediction_id,
            feedback_type,
            comment,
            metadata,
            created_by,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersion {
    pub id: String,
    pub model_id: String,
    pub version: String,
    pub description: Option<String>,
    pub checksum: Option<String>,
    pub path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl ModelVersion {
    pub fn new(
        model_id: String,
        version: String,
        description: Option<String>,
        checksum: Option<String>,
        path: Option<String>,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            model_id,
            version,
            description,
            checksum,
            path,
            created_at: Utc::now(),
            is_active: true,
            metadata,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    pub learning_rate: f64,
    pub batch_size: usize,
    pub max_epochs: usize,
    pub early_stopping_patience: Option<usize>,
    pub validation_split: f64,
    pub enable_online_learning: bool,
    pub enable_feedback_learning: bool,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            batch_size: 32,
            max_epochs: 100,
            early_stopping_patience: Some(10),
            validation_split: 0.2,
            enable_online_learning: true,
            enable_feedback_learning: true,
        }
    }
}

#[async_trait::async_trait]
pub trait AdaptiveLearner: Send + Sync {
    fn model_id(&self) -> &str;
    fn current_version(&self) -> &ModelVersion;
    fn config(&self) -> &LearningConfig;

    async fn predict(&self, input: &serde_json::Value) -> crate::utils::Result<serde_json::Value>;
    async fn update(
        &mut self,
        data: &[(serde_json::Value, serde_json::Value)],
    ) -> crate::utils::Result<()>;
    async fn apply_feedback(&mut self, feedback: &Feedback) -> crate::utils::Result<()>;
    async fn save_checkpoint(&self) -> crate::utils::Result<String>;
    async fn load_checkpoint(&mut self, checkpoint_path: &str) -> crate::utils::Result<()>;
}

pub use ab_testing::{ABTest, ABTestManager, ABTestResult, ABTestStatus, VersionStats};
pub use feedback::{FeedbackManager, FeedbackStats};
pub use monitoring::{
    AlertSeverity, DriftDetectionResult, ModelPerformanceMonitor, MonitoringConfig,
    PerformanceAlert, PerformanceMetrics,
};
pub use online::OnlineLearner;
pub use rollback::{AutoRollbackManager, RollbackEvent, RollbackPolicy, RollbackTrigger};
pub use versioning::{ModelVersionManager, VersionComparison};
