use crate::memory::{LongTermMemory, ShortTermMemory};
use crate::skill::Skill;
use crate::utils::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillState {
    pub skill_id: String,
    pub version: String,
    pub state_data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub checksum: String,
}

impl SkillState {
    pub fn new(skill_id: String, version: String, state_data: serde_json::Value) -> Self {
        let now = Utc::now();
        let checksum = Self::calculate_checksum(&state_data);

        Self {
            skill_id,
            version,
            state_data,
            created_at: now,
            updated_at: now,
            checksum,
        }
    }

    pub fn calculate_checksum(data: &serde_json::Value) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.to_string().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    pub fn update_state(&mut self, new_data: serde_json::Value) {
        self.state_data = new_data;
        self.updated_at = Utc::now();
        self.checksum = Self::calculate_checksum(&self.state_data);
    }

    pub fn is_valid(&self) -> bool {
        self.checksum == Self::calculate_checksum(&self.state_data)
    }
}

#[async_trait]
pub trait StatefulSkill: Skill {
    fn get_state(&self) -> Result<Option<SkillState>>;

    fn restore_state(&mut self, state: SkillState) -> Result<()>;

    fn with_memory(
        &mut self,
        _short_term: Option<Arc<ShortTermMemory>>,
        _long_term: Option<Arc<LongTermMemory>>,
    ) -> Result<()> {
        Ok(())
    }
}

pub struct BaseStatefulSkill {
    metadata: crate::skill::SkillMetadata,
    executor: Option<Box<dyn Fn(serde_json::Value) -> Result<serde_json::Value> + Send + Sync>>,
    current_state: Option<SkillState>,
    short_term_memory: Option<Arc<ShortTermMemory>>,
    long_term_memory: Option<Arc<LongTermMemory>>,
}

impl BaseStatefulSkill {
    pub fn new(metadata: crate::skill::SkillMetadata) -> Self {
        Self {
            metadata,
            executor: None,
            current_state: None,
            short_term_memory: None,
            long_term_memory: None,
        }
    }

    pub fn new_arc(metadata: crate::skill::SkillMetadata) -> Arc<dyn StatefulSkill> {
        Arc::new(Self::new(metadata))
    }

    pub fn with_executor<F>(mut self, executor: F) -> Self
    where
        F: Fn(serde_json::Value) -> Result<serde_json::Value> + Send + Sync + 'static,
    {
        self.executor = Some(Box::new(executor));
        self
    }

    pub fn short_term_memory(&self) -> Option<&Arc<ShortTermMemory>> {
        self.short_term_memory.as_ref()
    }

    pub fn long_term_memory(&self) -> Option<&Arc<LongTermMemory>> {
        self.long_term_memory.as_ref()
    }
}

#[async_trait]
impl Skill for BaseStatefulSkill {
    fn metadata(&self) -> &crate::skill::SkillMetadata {
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

#[async_trait]
impl StatefulSkill for BaseStatefulSkill {
    fn get_state(&self) -> Result<Option<SkillState>> {
        Ok(self.current_state.clone())
    }

    fn restore_state(&mut self, state: SkillState) -> Result<()> {
        if !state.is_valid() {
            warn!(
                "State checksum validation failed for skill: {}",
                state.skill_id
            );
            return Err(crate::utils::AetherisError::Skill(
                "State checksum validation failed".to_string(),
            ));
        }

        debug!("Restoring state for skill: {}", state.skill_id);
        self.current_state = Some(state);
        Ok(())
    }

    fn with_memory(
        &mut self,
        short_term: Option<Arc<ShortTermMemory>>,
        long_term: Option<Arc<LongTermMemory>>,
    ) -> Result<()> {
        debug!("Binding memory modules to skill: {}", self.metadata.id);
        self.short_term_memory = short_term;
        self.long_term_memory = long_term;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{SkillMetadata, Version};

    #[test]
    fn test_skill_state_new() {
        let state_data = serde_json::json!({"key": "value"});
        let state = SkillState::new(
            "test-skill".to_string(),
            "1.0.0".to_string(),
            state_data.clone(),
        );

        assert_eq!(state.skill_id, "test-skill");
        assert_eq!(state.version, "1.0.0");
        assert_eq!(state.state_data, state_data);
        assert!(state.is_valid());
    }

    #[test]
    fn test_skill_state_update() {
        let initial_data = serde_json::json!({"key": "value"});
        let mut state =
            SkillState::new("test-skill".to_string(), "1.0.0".to_string(), initial_data);

        let new_data = serde_json::json!({"key": "new_value"});
        state.update_state(new_data.clone());

        assert_eq!(state.state_data, new_data);
        assert!(state.is_valid());
    }

    #[test]
    fn test_skill_state_invalid_checksum() {
        let state_data = serde_json::json!({"key": "value"});
        let mut state = SkillState::new("test-skill".to_string(), "1.0.0".to_string(), state_data);

        state.checksum = "invalid".to_string();
        assert!(!state.is_valid());
    }

    #[test]
    fn test_base_stateful_skill_new() {
        let version = Version::new(1, 0, 0);
        let metadata = SkillMetadata::new(
            "test-skill".to_string(),
            "Test Skill".to_string(),
            version,
            "A test skill".to_string(),
        );

        let skill = BaseStatefulSkill::new(metadata);
        assert_eq!(skill.metadata().id, "test-skill");
    }

    #[tokio::test]
    async fn test_base_stateful_skill_execute() {
        let version = Version::new(1, 0, 0);
        let metadata = SkillMetadata::new(
            "test-skill".to_string(),
            "Test Skill".to_string(),
            version,
            "A test skill".to_string(),
        );

        let skill = BaseStatefulSkill::new(metadata)
            .with_executor(|input| Ok(serde_json::json!({"result": input})));

        let input = serde_json::json!({"test": "data"});
        let result = skill.execute(input.clone()).await.unwrap();

        assert_eq!(result, serde_json::json!({"result": input}));
    }

    #[test]
    fn test_stateful_skill_get_and_restore_state() {
        let version = Version::new(1, 0, 0);
        let metadata = SkillMetadata::new(
            "test-skill".to_string(),
            "Test Skill".to_string(),
            version,
            "A test skill".to_string(),
        );

        let mut skill = BaseStatefulSkill::new(metadata);

        assert!(skill.get_state().unwrap().is_none());

        let state_data = serde_json::json!({"key": "value"});
        let state = SkillState::new(
            "test-skill".to_string(),
            "1.0.0".to_string(),
            state_data.clone(),
        );

        skill.restore_state(state.clone()).unwrap();

        let retrieved = skill.get_state().unwrap().unwrap();
        assert_eq!(retrieved.state_data, state_data);
    }
}
