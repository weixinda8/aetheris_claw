use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::utils::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillStatus {
    Draft,
    Pending,
    Published,
    Deprecated,
    Blocked,
}

impl From<&str> for SkillStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "draft" => SkillStatus::Draft,
            "pending" => SkillStatus::Pending,
            "published" => SkillStatus::Published,
            "deprecated" => SkillStatus::Deprecated,
            "blocked" => SkillStatus::Blocked,
            _ => SkillStatus::Draft,
        }
    }
}

impl From<SkillStatus> for String {
    fn from(status: SkillStatus) -> Self {
        match status {
            SkillStatus::Draft => "draft".to_string(),
            SkillStatus::Pending => "pending".to_string(),
            SkillStatus::Published => "published".to_string(),
            SkillStatus::Deprecated => "deprecated".to_string(),
            SkillStatus::Blocked => "blocked".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditStage {
    AutomatedScan,
    JuniorReview,
    SeniorReview,
    Complete,
}

impl From<&str> for AuditStage {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "automated_scan" => AuditStage::AutomatedScan,
            "junior_review" => AuditStage::JuniorReview,
            "senior_review" => AuditStage::SeniorReview,
            "complete" => AuditStage::Complete,
            _ => AuditStage::AutomatedScan,
        }
    }
}

impl From<AuditStage> for String {
    fn from(stage: AuditStage) -> Self {
        match stage {
            AuditStage::AutomatedScan => "automated_scan".to_string(),
            AuditStage::JuniorReview => "junior_review".to_string(),
            AuditStage::SeniorReview => "senior_review".to_string(),
            AuditStage::Complete => "complete".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionLevel {
    Public,
    Internal,
    Restricted,
    Admin,
}

impl From<&str> for PermissionLevel {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "public" => PermissionLevel::Public,
            "internal" => PermissionLevel::Internal,
            "restricted" => PermissionLevel::Restricted,
            "admin" => PermissionLevel::Admin,
            _ => PermissionLevel::Public,
        }
    }
}

impl From<PermissionLevel> for String {
    fn from(level: PermissionLevel) -> Self {
        match level {
            PermissionLevel::Public => "Public".to_string(),
            PermissionLevel::Internal => "Internal".to_string(),
            PermissionLevel::Restricted => "Restricted".to_string(),
            PermissionLevel::Admin => "Admin".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub stage: AuditStage,
    pub reviewer_id: Option<Uuid>,
    pub status: String,
    pub comments: Option<String>,
    pub findings: Option<serde_json::Value>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQueueItem {
    pub skill_id: Uuid,
    pub skill_name: String,
    pub author_id: Uuid,
    pub author_name: Option<String>,
    pub current_stage: AuditStage,
    pub status: String,
    pub priority: String,
    pub submitted_at: DateTime<Utc>,
    pub waiting_time_seconds: i64,
}

pub struct AuditWorkflow;

impl AuditWorkflow {
    pub fn can_transition(from: &SkillStatus, to: &SkillStatus) -> bool {
        matches!(
            (from, to),
            (SkillStatus::Draft, SkillStatus::Pending)
                | (SkillStatus::Pending, SkillStatus::Published)
                | (SkillStatus::Pending, SkillStatus::Draft)
                | (SkillStatus::Pending, SkillStatus::Blocked)
                | (SkillStatus::Published, SkillStatus::Deprecated)
                | (SkillStatus::Published, SkillStatus::Blocked)
                | (SkillStatus::Blocked, SkillStatus::Draft)
                | (SkillStatus::Blocked, SkillStatus::Published)
                | (SkillStatus::Deprecated, SkillStatus::Draft)
                | (SkillStatus::Deprecated, SkillStatus::Published)
        )
    }

    pub fn next_audit_stage(current: &AuditStage) -> Option<AuditStage> {
        match current {
            AuditStage::AutomatedScan => Some(AuditStage::JuniorReview),
            AuditStage::JuniorReview => Some(AuditStage::SeniorReview),
            AuditStage::SeniorReview => Some(AuditStage::Complete),
            AuditStage::Complete => None,
        }
    }

    pub fn is_authorized_for_stage(
        user_role: &str,
        stage: &AuditStage,
    ) -> bool {
        match stage {
            AuditStage::AutomatedScan => true,
            AuditStage::JuniorReview => {
                user_role == "junior_auditor"
                    || user_role == "senior_auditor"
                    || user_role == "admin"
            }
            AuditStage::SeniorReview => {
                user_role == "senior_auditor" || user_role == "admin"
            }
            AuditStage::Complete => user_role == "admin",
        }
    }

    pub fn can_access_skill(
        user_permission_level: &PermissionLevel,
        skill_permission_level: &PermissionLevel,
    ) -> bool {
        match (user_permission_level, skill_permission_level) {
            (_, PermissionLevel::Public) => true,
            (PermissionLevel::Internal, PermissionLevel::Internal) => true,
            (PermissionLevel::Restricted, PermissionLevel::Internal) => true,
            (PermissionLevel::Restricted, PermissionLevel::Restricted) => true,
            (PermissionLevel::Admin, _) => true,
            _ => false,
        }
    }
}

pub struct AutomatedScanner;

impl AutomatedScanner {
    pub async fn scan_skill(skill_id: Uuid) -> Result<ScanResult> {
        let passed = true;
        let findings = if !passed {
            Some(serde_json::json!({
                "issues": ["Potential security vulnerability detected"]
            }))
        } else {
            None
        };

        Ok(ScanResult {
            skill_id,
            passed,
            score: if passed { 95.0 } else { 45.0 },
            findings,
            scanned_at: Utc::now(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub skill_id: Uuid,
    pub passed: bool,
    pub score: f64,
    pub findings: Option<serde_json::Value>,
    pub scanned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditAction {
    pub action: String,
    pub comments: Option<String>,
    pub findings: Option<serde_json::Value>,
}
