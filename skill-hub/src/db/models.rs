use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build: Option<String>,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
            build: None,
        }
    }

    pub fn from_string(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 3 {
            return Err("Version must be in format major.minor.patch".to_string());
        }

        let major = parts[0].parse().map_err(|e| format!("Invalid major version: {}", e))?;
        let minor = parts[1].parse().map_err(|e| format!("Invalid minor version: {}", e))?;
        let patch = parts[2].parse().map_err(|e| format!("Invalid patch version: {}", e))?;

        Ok(Self::new(major, minor, patch))
    }

    pub fn to_string(&self) -> String {
        let mut s = format!("{}.{}.{}", self.major, self.minor, self.patch);
        if let Some(pre) = &self.pre_release {
            s.push('-');
            s.push_str(pre);
        }
        if let Some(build) = &self.build {
            s.push('+');
            s.push_str(build);
        }
        s
    }

    pub fn is_compatible_with(&self, other: &Version) -> bool {
        self.major == other.major && self.minor >= other.minor
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => match self.minor.cmp(&other.minor) {
                std::cmp::Ordering::Equal => self.patch.cmp(&other.patch),
                ord => ord,
            },
            ord => ord,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CallMode {
    Text,
    Api,
    Database,
    Image,
    Audio,
    Hybrid,
}

impl CallMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            CallMode::Text => "Text",
            CallMode::Api => "Api",
            CallMode::Database => "Database",
            CallMode::Image => "Image",
            CallMode::Audio => "Audio",
            CallMode::Hybrid => "Hybrid",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "Text" => Ok(CallMode::Text),
            "Api" => Ok(CallMode::Api),
            "Database" => Ok(CallMode::Database),
            "Image" => Ok(CallMode::Image),
            "Audio" => Ok(CallMode::Audio),
            "Hybrid" => Ok(CallMode::Hybrid),
            _ => Err(format!("Invalid call mode: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PermissionLevel {
    Public,
    Internal,
    Restricted,
    Admin,
}

impl PermissionLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionLevel::Public => "Public",
            PermissionLevel::Internal => "Internal",
            PermissionLevel::Restricted => "Restricted",
            PermissionLevel::Admin => "Admin",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "Public" => Ok(PermissionLevel::Public),
            "Internal" => Ok(PermissionLevel::Internal),
            "Restricted" => Ok(PermissionLevel::Restricted),
            "Admin" => Ok(PermissionLevel::Admin),
            _ => Err(format!("Invalid permission level: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkillPriority {
    Mandatory,
    High,
    Medium,
    Low,
    OnDemand,
    Disabled,
}

impl SkillPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillPriority::Mandatory => "Mandatory",
            SkillPriority::High => "High",
            SkillPriority::Medium => "Medium",
            SkillPriority::Low => "Low",
            SkillPriority::OnDemand => "OnDemand",
            SkillPriority::Disabled => "Disabled",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "Mandatory" => Ok(SkillPriority::Mandatory),
            "High" => Ok(SkillPriority::High),
            "Medium" => Ok(SkillPriority::Medium),
            "Low" => Ok(SkillPriority::Low),
            "OnDemand" => Ok(SkillPriority::OnDemand),
            "Disabled" => Ok(SkillPriority::Disabled),
            _ => Err(format!("Invalid skill priority: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub description: String,
    pub long_description: Option<String>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub author: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub call_mode: CallMode,
    pub permission_level: PermissionLevel,
    pub priority: SkillPriority,
    pub required_permissions: Vec<String>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub example_input: Option<serde_json::Value>,
    pub example_output: Option<serde_json::Value>,
    pub dependencies: Vec<String>,
    pub is_active: bool,
    pub is_deprecated: bool,
    pub deprecation_reason: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl SkillMetadata {
    pub fn new(id: String, name: String, version: Version, description: String) -> Self {
        Self {
            id,
            name,
            version,
            description,
            long_description: None,
            tags: Vec::new(),
            categories: Vec::new(),
            author: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            call_mode: CallMode::Text,
            permission_level: PermissionLevel::Public,
            priority: SkillPriority::Medium,
            required_permissions: Vec::new(),
            input_schema: None,
            output_schema: None,
            example_input: None,
            example_output: None,
            dependencies: Vec::new(),
            is_active: true,
            is_deprecated: false,
            deprecation_reason: None,
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadataExtended {
    pub base: SkillMetadata,
    pub hub_id: Uuid,
    pub status: String,
    pub author_id: Uuid,
    pub author_name: Option<String>,
    pub category: Option<String>,
    pub download_count: i64,
    pub average_rating: f64,
    pub rating_count: i32,
    pub success_rate: f64,
    pub execution_count: i64,
    pub published_at: Option<DateTime<Utc>>,
    pub deprecated_at: Option<DateTime<Utc>>,
}

impl SkillMetadataExtended {
    pub fn from_base(
        base: SkillMetadata,
        hub_id: Uuid,
        status: String,
        author_id: Uuid,
    ) -> Self {
        Self {
            base,
            hub_id,
            status,
            author_id,
            author_name: None,
            category: None,
            download_count: 0,
            average_rating: 0.0,
            rating_count: 0,
            success_rate: 0.0,
            execution_count: 0,
            published_at: None,
            deprecated_at: None,
        }
    }
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
pub struct SkillDownloadRecord {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub user_id: Option<Uuid>,
    pub downloaded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionRecord {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub version: String,
    pub success: bool,
    pub executed_at: DateTime<Utc>,
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
