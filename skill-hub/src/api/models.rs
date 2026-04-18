use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::utils::audit::{AuditStage, AuditRecord, AuditQueueItem, ScanResult, SkillStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub author_id: Uuid,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSkillRequest {
    pub skill_id: String,
    pub name: String,
    pub description: String,
    pub long_description: Option<String>,
    pub version: String,
    pub category: Option<String>,
    pub categories: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub call_mode: Option<String>,
    pub permission_level: Option<String>,
    pub priority: Option<String>,
    pub required_permissions: Option<Vec<String>>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub example_input: Option<serde_json::Value>,
    pub example_output: Option<serde_json::Value>,
    pub dependencies: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
    pub content: serde_json::Value,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSkillRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub long_description: Option<String>,
    pub version: Option<String>,
    pub category: Option<String>,
    pub categories: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub status: Option<String>,
    pub call_mode: Option<String>,
    pub permission_level: Option<String>,
    pub priority: Option<String>,
    pub required_permissions: Option<Vec<String>>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub example_input: Option<serde_json::Value>,
    pub example_output: Option<serde_json::Value>,
    pub dependencies: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
    pub content: Option<serde_json::Value>,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub version: String,
    pub content: serde_json::Value,
    pub changelog: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReviewRequest {
    pub skill_id: Uuid,
    pub rating: i32,
    pub title: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub sort_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewListResponse {
    pub reviews: Vec<SkillReview>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSearchQuery {
    pub query: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<String>,
    pub sort_by: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSearchResponse {
    pub skills: Vec<Skill>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSkillRequest {
    pub skill_id: Uuid,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSkillResponse {
    pub skill_id: Uuid,
    pub version: String,
    pub content: serde_json::Value,
    pub downloaded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordExecutionRequest {
    pub skill_id: Uuid,
    pub version: String,
    pub success: bool,
    pub execution_time_ms: Option<i64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStats {
    pub total_skills: i64,
    pub published_skills: i64,
    pub total_downloads: i64,
    pub total_users: i64,
    pub total_reviews: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub username: String,
    pub role: String,
    pub exp: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSkillResponse {
    pub skill_id: Uuid,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRatingResponse {
    pub skill_id: Uuid,
    pub bayesian_rating: f64,
    pub normalized_downloads: f64,
    pub smoothed_success_rate: f64,
    pub activity_score: f64,
    pub overall_score: f64,
    pub audit_quality_score: f64,
    pub trending_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillWithRating {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub author_id: Uuid,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: String,
    pub download_count: i64,
    pub average_rating: f64,
    pub rating_count: i32,
    pub success_rate: f64,
    pub execution_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub deprecated_at: Option<DateTime<Utc>>,
    pub rating: Option<SkillRatingResponse>,
}

impl From<Skill> for SkillWithRating {
    fn from(skill: Skill) -> Self {
        Self {
            id: skill.id,
            name: skill.name,
            description: skill.description,
            version: skill.version,
            author_id: skill.author_id,
            category: skill.category,
            tags: skill.tags,
            status: skill.status,
            download_count: skill.download_count,
            average_rating: skill.average_rating,
            rating_count: skill.rating_count,
            success_rate: skill.success_rate,
            execution_count: skill.execution_count,
            created_at: skill.created_at,
            updated_at: skill.updated_at,
            published_at: skill.published_at,
            deprecated_at: skill.deprecated_at,
            rating: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSearchWithRatingResponse {
    pub skills: Vec<SkillWithRating>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitForAuditRequest {
    pub comments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditActionRequest {
    pub action: String,
    pub comments: Option<String>,
    pub findings: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQueueQuery {
    pub stage: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQueueResponse {
    pub items: Vec<AuditQueueItem>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAuditHistoryResponse {
    pub skill_id: Uuid,
    pub records: Vec<AuditRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSkillStatusRequest {
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSkillPermissionRequest {
    pub permission_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatedScanResponse {
    pub skill_id: Uuid,
    pub scan_result: ScanResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStatsResponse {
    pub total_pending: i64,
    pub in_automated_scan: i64,
    pub in_junior_review: i64,
    pub in_senior_review: i64,
    pub completed_today: i64,
    pub average_wait_time_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSkillVersionRequest {
    pub version: String,
    pub content: serde_json::Value,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSkillVersionResponse {
    pub version_id: Uuid,
    pub skill_id: Uuid,
    pub version: String,
}
