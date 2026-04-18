use crate::agent::config::config::{AgentConfig, AgentConfigError, AgentPersona};
use crate::soul::{Soul, SoulRegistry};

pub struct SoulIntegrationManager {
    soul_registry: Option<SoulRegistry>,
}

impl SoulIntegrationManager {
    pub fn new() -> Self {
        Self {
            soul_registry: None,
        }
    }

    pub fn with_soul_registry(mut self, registry: SoulRegistry) -> Self {
        self.soul_registry = Some(registry);
        self
    }

    pub async fn load_soul_for_agent(
        &self,
        config: &mut AgentConfig,
    ) -> Result<Option<Soul>, AgentConfigError> {
        let Some(soul_path) = &config.persona.soul_file else {
            return Ok(None);
        };

        let soul = if let Some(registry) = &self.soul_registry {
            if let Some(soul) = registry.get_by_path(soul_path) {
                soul.clone()
            } else {
                Soul::from_path(soul_path.clone()).map_err(|e| {
                    AgentConfigError::Validation(format!("Failed to load soul: {}", e))
                })?
            }
        } else {
            Soul::from_path(soul_path.clone())
                .map_err(|e| AgentConfigError::Validation(format!("Failed to load soul: {}", e)))?
        };

        if config.persona.system_prompt.is_none() {
            config.persona.system_prompt = Some(soul.system_prompt());
        }

        Ok(Some(soul))
    }

    pub fn apply_soul_to_persona(soul: &Soul, persona: &mut AgentPersona) {
        persona.system_prompt = Some(soul.system_prompt());

        if let Some(ref mut personality_config) = persona.personality {
            personality_config.style = Some(soul.metadata.personality.clone());
        }
    }

    pub async fn create_agent_from_soul(
        &self,
        soul_name: &str,
        base_config: AgentConfig,
    ) -> Result<AgentConfig, AgentConfigError> {
        let Some(registry) = &self.soul_registry else {
            return Err(AgentConfigError::Validation(
                "Soul registry not initialized".to_string(),
            ));
        };

        let Some(soul) = registry.get(soul_name) else {
            return Err(AgentConfigError::Validation(format!(
                "Soul not found: {}",
                soul_name
            )));
        };

        let mut config = base_config;
        config.persona.soul_file = Some(soul.path.clone());
        config.persona.system_prompt = Some(soul.system_prompt());

        Ok(config)
    }

    pub fn list_available_souls(&self) -> Vec<String> {
        if let Some(registry) = &self.soul_registry {
            registry
                .list()
                .iter()
                .map(|s| s.name().to_string())
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for SoulIntegrationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soul_integration_manager_creation() {
        let manager = SoulIntegrationManager::new();
        assert!(manager.soul_registry.is_none());
    }

    #[test]
    fn test_list_available_souls_empty() {
        let manager = SoulIntegrationManager::new();
        let souls = manager.list_available_souls();
        assert!(souls.is_empty());
    }
}
