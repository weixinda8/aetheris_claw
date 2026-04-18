use crate::agent::config::config::{AgentConfig, AgentConfigError};
use crate::skill::agentskills::{AgentSkillsRegistry, SkillMdDocument};
use crate::skill::{Skill, SkillRegistry};

pub struct SkillIntegrationManager {
    skill_registry: Option<SkillRegistry>,
    agent_skills_registry: Option<AgentSkillsRegistry>,
}

impl SkillIntegrationManager {
    pub fn new() -> Self {
        Self {
            skill_registry: None,
            agent_skills_registry: None,
        }
    }

    pub fn with_skill_registry(mut self, registry: SkillRegistry) -> Self {
        self.skill_registry = Some(registry);
        self
    }

    pub fn with_agent_skills_registry(mut self, registry: AgentSkillsRegistry) -> Self {
        self.agent_skills_registry = Some(registry);
        self
    }

    pub fn validate_skills_for_agent(&self, config: &AgentConfig) -> Result<(), AgentConfigError> {
        if config.skills.enabled.is_empty() {
            return Ok(());
        }

        for skill_id in &config.skills.enabled {
            let found_in_old = self
                .skill_registry
                .as_ref()
                .and_then(|r| r.get(skill_id))
                .is_some();
            let found_in_agent = self
                .agent_skills_registry
                .as_ref()
                .and_then(|r| r.get(skill_id))
                .is_some();

            if !found_in_old && !found_in_agent {
                return Err(AgentConfigError::Validation(format!(
                    "Skill not found: {}",
                    skill_id
                )));
            }
        }

        Ok(())
    }

    pub fn get_enabled_skills(&self, config: &AgentConfig) -> Vec<std::sync::Arc<dyn Skill>> {
        let mut skills = Vec::new();

        if config.skills.enabled.is_empty() {
            return skills;
        }

        for skill_id in &config.skills.enabled {
            if let Some(registry) = &self.skill_registry {
                if let Some(skill) = registry.get(skill_id) {
                    skills.push(skill);
                    continue;
                }
            }

            if let Some(agent_registry) = &self.agent_skills_registry {
                if let Some(_manifest) = agent_registry.get(skill_id) {}
            }
        }

        skills
    }

    pub fn list_available_skills(&self) -> Vec<String> {
        let mut skills = Vec::new();

        if let Some(registry) = &self.skill_registry {
            skills.extend(registry.list().iter().map(|m| m.id.clone()));
        }

        if let Some(agent_registry) = &self.agent_skills_registry {
            skills.extend(agent_registry.list().iter().map(|m| m.metadata.id.clone()));
        }

        skills
    }

    pub fn load_skill_from_skill_md(
        &self,
        path: &str,
    ) -> Result<Option<SkillMdDocument>, AgentConfigError> {
        if let Some(_agent_registry) = &self.agent_skills_registry {
            let doc = SkillMdDocument::from_path(path, true).map_err(|e| {
                AgentConfigError::Validation(format!("Failed to load SKILL.md: {}", e))
            })?;
            Ok(Some(doc))
        } else {
            Ok(None)
        }
    }

    pub fn apply_skill_permissions(config: &AgentConfig, skill_id: &str) -> bool {
        if let Some(permissions) = &config.skills.permissions {
            permissions.contains(&skill_id.to_string())
        } else {
            true
        }
    }
}

impl Default for SkillIntegrationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::agentskills::AgentSkillsRegistry;
    use crate::skill::{BaseSkill, SkillMetadata, Version};
    use tempfile::tempdir;

    #[test]
    fn test_skill_integration_manager_creation() {
        let manager = SkillIntegrationManager::new();
        assert!(manager.skill_registry.is_none());
        assert!(manager.agent_skills_registry.is_none());
    }

    #[test]
    fn test_list_available_skills_empty() {
        let manager = SkillIntegrationManager::new();
        let skills = manager.list_available_skills();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_with_agent_skills_registry() {
        let agent_registry = AgentSkillsRegistry::new();
        let manager = SkillIntegrationManager::new().with_agent_skills_registry(agent_registry);
        assert!(manager.agent_skills_registry.is_some());
    }

    #[test]
    fn test_list_available_skills_with_both_registries() {
        let mut skill_registry = crate::skill::SkillRegistry::new();
        let metadata = SkillMetadata::new(
            "old-skill".to_string(),
            "Old Skill".to_string(),
            Version::new(1, 0, 0),
            "Old skill description".to_string(),
        );
        let skill = BaseSkill::new_arc(metadata);
        skill_registry.register(skill);

        let agent_registry = AgentSkillsRegistry::new();

        let manager = SkillIntegrationManager::new()
            .with_skill_registry(skill_registry)
            .with_agent_skills_registry(agent_registry);

        let skills = manager.list_available_skills();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0], "old-skill");
    }

    #[test]
    fn test_validate_skills_for_agent_empty() {
        let manager = SkillIntegrationManager::new();
        let config = crate::agent::config::config::AgentConfig::default();
        let result = manager.validate_skills_for_agent(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_enabled_skills_empty() {
        let manager = SkillIntegrationManager::new();
        let config = crate::agent::config::config::AgentConfig::default();
        let skills = manager.get_enabled_skills(&config);
        assert!(skills.is_empty());
    }

    #[test]
    fn test_with_both_registries() {
        let skill_registry = crate::skill::SkillRegistry::new();
        let agent_registry = AgentSkillsRegistry::new();

        let manager = SkillIntegrationManager::new()
            .with_skill_registry(skill_registry)
            .with_agent_skills_registry(agent_registry);

        assert!(manager.skill_registry.is_some());
        assert!(manager.agent_skills_registry.is_some());
    }

    #[test]
    fn test_apply_skill_permissions_no_permissions() {
        let config = crate::agent::config::config::AgentConfig::default();
        let has_permission =
            SkillIntegrationManager::apply_skill_permissions(&config, "test-skill");
        assert!(has_permission);
    }
}
