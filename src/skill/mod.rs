pub mod agentskills;
pub mod clawhub;
pub mod intent_matching;
pub mod loader;
pub mod metadata_index;
pub mod oci_registry;
pub mod progressive_disclosure;
pub mod registry;
pub mod skill_cache;
pub mod skill_state_manager;
pub mod stateful_skill;
pub mod sub_skill_manager;
pub mod tool_discovery;
pub mod unified_call;

pub use agentskills::{
    AgentSkillExample, AgentSkillManifest, AgentSkillMetadata, AgentSkillParameter,
    AgentSkillRetryConfig, AgentSkillReturn, AgentSkillType, AgentSkillsRegistry, SkillMdDocument,
    SkillMdFrontmatter, SkillMdSections,
};

pub use clawhub::{
    AetherisSkillHubClient, AetherisSkillHubSearchResult, AetherisSkillHubSkillInfo, ClawHubClient,
    ClawHubImporter, ClawHubSearchResult, ClawHubSkillInfo, ClawHubSync, CreateReviewRequest,
    RecordExecutionRequest, RetryConfig, SkillReview, SkillSource, SkillUpdateCheck,
    UnifiedSearchResult, UnifiedSkillHubClient, UnifiedSkillInfo,
};
pub use intent_matching::*;
pub use loader::{SkillConfigFile, SkillLoader};
pub use metadata_index::*;
pub use progressive_disclosure::*;
pub use registry::SkillRegistry;
pub use skill_cache::*;
pub use skill_state_manager::{SkillStateManager, SkillStateVersion, StateSnapshot};
pub use stateful_skill::{BaseStatefulSkill, SkillState, StatefulSkill};
pub use sub_skill_manager::{SubSkillManager, SubSkillRelationship};

use crate::utils::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

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

    pub fn from_string(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 3 {
            return Err(crate::utils::AetherisError::Skill(
                "Version must be in format major.minor.patch".to_string(),
            ));
        }

        let major = parts[0].parse().map_err(|e| {
            crate::utils::AetherisError::Skill(format!("Invalid major version: {}", e))
        })?;
        let minor = parts[1].parse().map_err(|e| {
            crate::utils::AetherisError::Skill(format!("Invalid minor version: {}", e))
        })?;
        let patch = parts[2].parse().map_err(|e| {
            crate::utils::AetherisError::Skill(format!("Invalid patch version: {}", e))
        })?;

        Ok(Self::new(major, minor, patch))
    }

    pub fn is_compatible_with(&self, other: &Version) -> bool {
        self.major == other.major && self.minor >= other.minor
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre_release {
            write!(f, "-{}", pre)?;
        }
        if let Some(build) = &self.build {
            write!(f, "+{}", build)?;
        }
        Ok(())
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PermissionLevel {
    Public,
    Internal,
    Restricted,
    Admin,
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
            SkillPriority::Mandatory => "mandatory",
            SkillPriority::High => "high",
            SkillPriority::Medium => "medium",
            SkillPriority::Low => "low",
            SkillPriority::OnDemand => "ondemand",
            SkillPriority::Disabled => "disabled",
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            SkillPriority::Mandatory => 1,
            SkillPriority::High => 2,
            SkillPriority::Medium => 3,
            SkillPriority::Low => 4,
            SkillPriority::OnDemand => 5,
            SkillPriority::Disabled => 6,
        }
    }

    pub fn should_load(&self) -> bool {
        !matches!(self, SkillPriority::Disabled)
    }

    pub fn should_preload(&self) -> bool {
        matches!(
            self,
            SkillPriority::Mandatory | SkillPriority::High | SkillPriority::Medium
        )
    }

    pub fn is_lazy_load(&self) -> bool {
        matches!(self, SkillPriority::Low | SkillPriority::OnDemand)
    }
}

impl std::str::FromStr for SkillPriority {
    type Err = crate::utils::AetherisError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mandatory" | "1" => Ok(SkillPriority::Mandatory),
            "high" | "2" => Ok(SkillPriority::High),
            "medium" | "3" => Ok(SkillPriority::Medium),
            "low" | "4" => Ok(SkillPriority::Low),
            "ondemand" | "on_demand" | "5" => Ok(SkillPriority::OnDemand),
            "disabled" | "6" => Ok(SkillPriority::Disabled),
            _ => Err(crate::utils::AetherisError::Skill(format!(
                "Invalid skill priority: {}",
                s
            ))),
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

    pub fn with_priority(mut self, priority: SkillPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_call_mode(mut self, mode: CallMode) -> Self {
        self.call_mode = mode;
        self
    }

    pub fn with_permission_level(mut self, level: PermissionLevel) -> Self {
        self.permission_level = level;
        self
    }

    pub fn with_input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    pub fn with_output_schema(mut self, schema: serde_json::Value) -> Self {
        self.output_schema = Some(schema);
        self
    }
}

#[async_trait]
pub trait Skill: Send + Sync {
    fn metadata(&self) -> &SkillMetadata;
    async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value>;
    async fn validate_input(&self, _input: &serde_json::Value) -> Result<bool> {
        Ok(true)
    }
    async fn validate_output(&self, _output: &serde_json::Value) -> Result<bool> {
        Ok(true)
    }
}

impl dyn Skill {
    pub fn from_arc<T: Skill + 'static>(skill: T) -> Arc<Self> {
        Arc::new(skill) as Arc<Self>
    }
}

pub struct BaseSkill {
    metadata: SkillMetadata,
    executor: Option<Box<dyn Fn(serde_json::Value) -> Result<serde_json::Value> + Send + Sync>>,
}

impl BaseSkill {
    pub fn new(metadata: SkillMetadata) -> Self {
        Self {
            metadata,
            executor: None,
        }
    }

    pub fn new_arc(metadata: SkillMetadata) -> Arc<dyn Skill> {
        <dyn Skill>::from_arc(Self::new(metadata))
    }

    pub fn with_executor<F>(mut self, executor: F) -> Self
    where
        F: Fn(serde_json::Value) -> Result<serde_json::Value> + Send + Sync + 'static,
    {
        self.executor = Some(Box::new(executor));
        self
    }
}

#[async_trait]
impl Skill for BaseSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value> {
        if let Some(executor) = &self.executor {
            executor(input)
        } else {
            Ok(serde_json::Value::Null)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEvaluation {
    pub skill_id: String,
    pub version: Version,
    pub relevance_score: f64,
    pub performance_score: f64,
    pub reliability_score: f64,
    pub overall_score: f64,
    pub evaluation_time: DateTime<Utc>,
    pub evaluation_criteria: HashMap<String, String>,
}

impl SkillEvaluation {
    pub fn new(skill_id: String, version: Version) -> Self {
        Self {
            skill_id,
            version,
            relevance_score: 0.0,
            performance_score: 0.0,
            reliability_score: 0.0,
            overall_score: 0.0,
            evaluation_time: Utc::now(),
            evaluation_criteria: HashMap::new(),
        }
    }

    pub fn calculate_overall(&mut self) {
        self.overall_score = (self.relevance_score * 0.4)
            + (self.performance_score * 0.3)
            + (self.reliability_score * 0.3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_new() {
        let version = Version::new(1, 2, 3);
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_version_from_string() {
        let version = Version::from_string("1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_version_from_string_invalid() {
        let result = Version::from_string("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_version_to_string() {
        let version = Version::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 1, 0);
        let v3 = Version::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }

    #[test]
    fn test_version_compatibility() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 1, 0);
        let v3 = Version::new(2, 0, 0);

        assert!(v2.is_compatible_with(&v1));
        assert!(!v1.is_compatible_with(&v2));
        assert!(!v3.is_compatible_with(&v1));
    }

    #[test]
    fn test_skill_priority_from_str() {
        assert_eq!(
            "high".parse::<SkillPriority>().unwrap(),
            SkillPriority::High
        );
        assert_eq!(
            "medium".parse::<SkillPriority>().unwrap(),
            SkillPriority::Medium
        );
        assert_eq!("low".parse::<SkillPriority>().unwrap(), SkillPriority::Low);
    }

    #[test]
    fn test_skill_priority_as_str() {
        assert_eq!(SkillPriority::High.as_str(), "high");
        assert_eq!(SkillPriority::Medium.as_str(), "medium");
    }

    #[test]
    fn test_skill_priority_should_load() {
        assert!(SkillPriority::Mandatory.should_load());
        assert!(SkillPriority::High.should_load());
        assert!(!SkillPriority::Disabled.should_load());
    }

    #[test]
    fn test_skill_priority_should_preload() {
        assert!(SkillPriority::Mandatory.should_preload());
        assert!(SkillPriority::High.should_preload());
        assert!(!SkillPriority::OnDemand.should_preload());
    }

    #[test]
    fn test_skill_metadata_new() {
        let version = Version::new(1, 0, 0);
        let metadata = SkillMetadata::new(
            "test-skill".to_string(),
            "Test Skill".to_string(),
            version,
            "A test skill".to_string(),
        );
        assert_eq!(metadata.id, "test-skill");
        assert_eq!(metadata.name, "Test Skill");
        assert_eq!(metadata.is_active, true);
    }

    #[test]
    fn test_skill_evaluation_calculate_overall() {
        let version = Version::new(1, 0, 0);
        let mut evaluation = SkillEvaluation::new("test-skill".to_string(), version);
        evaluation.relevance_score = 0.8;
        evaluation.performance_score = 0.9;
        evaluation.reliability_score = 0.7;
        evaluation.calculate_overall();
        let expected = 0.8 * 0.4 + 0.9 * 0.3 + 0.7 * 0.3;
        assert!((evaluation.overall_score - expected).abs() < 0.0001);
    }
}
