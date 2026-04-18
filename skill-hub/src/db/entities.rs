use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Skill {
    pub id: Uuid,
    pub skill_id: String,
    pub name: String,
    pub description: String,
    pub long_description: Option<String>,
    pub version: String,
    pub author_id: Uuid,
    pub author_name: Option<String>,
    pub category: Option<String>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub status: String,
    pub call_mode: String,
    pub permission_level: String,
    pub priority: String,
    pub required_permissions: Vec<String>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub example_input: Option<serde_json::Value>,
    pub example_output: Option<serde_json::Value>,
    pub dependencies: Vec<String>,
    pub is_active: bool,
    pub is_deprecated: bool,
    pub deprecation_reason: Option<String>,
    pub metadata: serde_json::Value,
    pub download_count: i64,
    pub average_rating: f64,
    pub rating_count: i32,
    pub success_rate: f64,
    pub execution_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub deprecated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillVersion {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub version: String,
    pub content: serde_json::Value,
    pub changelog: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillReview {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub user_id: Uuid,
    pub rating: i32,
    pub title: Option<String>,
    pub content: Option<String>,
    pub helpful_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillDownload {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub user_id: Option<Uuid>,
    pub downloaded_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillExecution {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub version: String,
    pub success: bool,
    pub executed_at: DateTime<Utc>,
    pub execution_time_ms: Option<i64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillCategory {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillTag {
    pub id: Uuid,
    pub name: String,
    pub usage_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReviewHelpfulness {
    pub id: Uuid,
    pub review_id: Uuid,
    pub user_id: Uuid,
    pub is_helpful: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillAuditLog {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub action: String,
    pub performed_by: Uuid,
    pub details: Option<serde_json::Value>,
    pub performed_at: DateTime<Utc>,
}
