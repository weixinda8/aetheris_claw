use crate::agent::base::{Agent, AgentRegistry};
use crate::agent::config::config::{
    AgentConfig as ConfigurableAgentConfig, AgentConfigError, GlobalAgentConfig,
};
use crate::agent::config::loader::AgentConfigLoader;
use crate::agent::config::template::{
    AgentTemplate, AgentTemplateEngine, TemplateError, create_default_templates,
};
use crate::agent::config_driven::ConfigDrivenAgent;
use crate::core::llm::manager::LlmManager;
use crate::core::progressive_loading::ProgressiveLoader;
use crate::memory::short_term::ShortTermMemory;
use crate::skill::agentskills::AgentSkillsRegistry;
use crate::skill::registry::SkillRegistry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum AgentFactoryError {
    #[error("Config error: {0}")]
    Config(#[from] AgentConfigError),
    #[error("Template error: {0}")]
    Template(#[from] TemplateError),
    #[error("Agent config not found: {0}")]
    ConfigNotFound(PathBuf),
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Agent creation failed: {0}")]
    CreationFailed(String),
    #[error("Registry error: {0}")]
    Registry(String),
}

pub struct AgentFactory {
    config_loader: AgentConfigLoader,
    template_engine: AgentTemplateEngine,
    llm_manager: Option<Arc<LlmManager>>,
    skill_registry: Option<Arc<SkillRegistry>>,
    agent_skills_registry: Option<Arc<AgentSkillsRegistry>>,
    short_term_memory: Option<Arc<ShortTermMemory>>,
    progressive_loader: Option<Arc<ProgressiveLoader>>,
    global_config: Option<GlobalAgentConfig>,
}

impl AgentFactory {
    pub fn new() -> Self {
        let mut template_engine = AgentTemplateEngine::new();

        for template in create_default_templates() {
            if let Err(e) = template_engine.register_template(template) {
                warn!("Failed to register default template: {}", e);
            }
        }

        Self {
            config_loader: AgentConfigLoader::new(),
            template_engine,
            llm_manager: None,
            skill_registry: None,
            agent_skills_registry: None,
            short_term_memory: None,
            progressive_loader: None,
            global_config: None,
        }
    }

    pub fn with_global_config(mut self, global_config: GlobalAgentConfig) -> Self {
        self.global_config = Some(global_config.clone());
        self.config_loader = self.config_loader.with_global_config(global_config);
        self
    }

    pub fn with_llm_manager(mut self, llm_manager: Arc<LlmManager>) -> Self {
        self.llm_manager = Some(llm_manager);
        self
    }

    pub fn with_skill_registry(mut self, skill_registry: Arc<SkillRegistry>) -> Self {
        self.skill_registry = Some(skill_registry);
        self
    }

    pub fn with_agent_skills_registry(
        mut self,
        agent_skills_registry: Arc<AgentSkillsRegistry>,
    ) -> Self {
        self.agent_skills_registry = Some(agent_skills_registry);
        self
    }

    pub fn with_short_term_memory(mut self, memory: Arc<ShortTermMemory>) -> Self {
        self.short_term_memory = Some(memory);
        self
    }

    pub fn with_progressive_loader(mut self, loader: Arc<ProgressiveLoader>) -> Self {
        self.progressive_loader = Some(loader);
        self
    }

    pub fn register_template(
        &mut self,
        template: AgentTemplate,
    ) -> std::result::Result<(), AgentFactoryError> {
        self.template_engine.register_template(template)?;
        Ok(())
    }

    pub fn load_templates_from_directory<P: Into<PathBuf>>(
        &mut self,
        dir: P,
    ) -> std::result::Result<Vec<AgentTemplate>, AgentFactoryError> {
        let templates = self.template_engine.load_templates_from_directory(dir)?;
        Ok(templates)
    }

    pub fn list_templates(&self) -> Vec<&AgentTemplate> {
        self.template_engine.list_templates()
    }

    pub fn create_agent_from_config(
        &self,
        config: ConfigurableAgentConfig,
    ) -> std::result::Result<Arc<dyn Agent + Send + Sync>, AgentFactoryError> {
        info!("Creating agent from config: {}", config.meta.id);

        let mut agent = ConfigDrivenAgent::new(config);

        if let Some(llm) = &self.llm_manager {
            agent = agent.with_llm_manager(llm.clone());
        }

        if let Some(skills) = &self.skill_registry {
            agent = agent.with_skill_registry(skills.clone());
        }

        if let Some(_agent_skills) = &self.agent_skills_registry {}

        if let Some(memory) = &self.short_term_memory {
            agent = agent.with_short_term_memory(memory.clone());
        }

        if let Some(loader) = &self.progressive_loader {
            agent = agent.with_progressive_loader(loader.clone());
        }

        Ok(<dyn Agent + Send + Sync>::from_arc(agent))
    }

    pub fn create_agent_from_file<P: Into<PathBuf>>(
        &self,
        config_path: P,
    ) -> std::result::Result<Arc<dyn Agent + Send + Sync>, AgentFactoryError> {
        let path = config_path.into();

        if !path.exists() {
            return Err(AgentFactoryError::ConfigNotFound(path));
        }

        info!("Loading agent from file: {:?}", path);

        let config = if let Some(ext) = path.extension() {
            match ext.to_str() {
                Some("yaml") | Some("yml") => self.config_loader.load_from_yaml_file(&path)?,
                Some("json5") => self.config_loader.load_from_json5_file(&path)?,
                _ => {
                    return Err(AgentFactoryError::CreationFailed(format!(
                        "Unsupported config format: {:?}",
                        ext
                    )));
                }
            }
        } else {
            self.config_loader.load_from_yaml_file(&path)?
        };

        self.create_agent_from_config(config)
    }

    pub fn create_agent_from_template(
        &self,
        template_id: &str,
        variables: &HashMap<String, String>,
        workspace: PathBuf,
    ) -> std::result::Result<Arc<dyn Agent + Send + Sync>, AgentFactoryError> {
        info!("Creating agent from template: {}", template_id);

        let config = self
            .template_engine
            .render_to_config(template_id, variables, workspace)?;

        self.create_agent_from_config(config)
    }

    pub fn load_all_agents_from_directory<P: Into<PathBuf>>(
        &self,
        dir: P,
    ) -> std::result::Result<Vec<Arc<dyn Agent + Send + Sync>>, AgentFactoryError> {
        let dir = dir.into();
        info!("Loading all agents from directory: {:?}", dir);

        let configs = self.config_loader.load_all_from_directory(&dir)?;
        let mut agents = Vec::new();

        for config in configs {
            match self.create_agent_from_config(config) {
                Ok(agent) => agents.push(agent),
                Err(e) => warn!("Failed to create agent: {}", e),
            }
        }

        Ok(agents)
    }

    pub fn register_all_agents_to_registry(
        &self,
        registry: &AgentRegistry,
        agents: Vec<Arc<dyn Agent + Send + Sync>>,
    ) -> std::result::Result<(), AgentFactoryError> {
        for agent in agents {
            registry.register_agent(agent).map_err(|e| {
                AgentFactoryError::Registry(format!("Failed to register agent: {}", e))
            })?;
        }
        Ok(())
    }

    pub fn create_and_register_agent_from_file<P: Into<PathBuf>>(
        &self,
        config_path: P,
        registry: &AgentRegistry,
    ) -> std::result::Result<Arc<dyn Agent + Send + Sync>, AgentFactoryError> {
        let agent = self.create_agent_from_file(config_path)?;
        registry
            .register_agent(agent.clone())
            .map_err(|e| AgentFactoryError::Registry(format!("Failed to register agent: {}", e)))?;
        Ok(agent)
    }
}

impl Default for AgentFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_creation() {
        let factory = AgentFactory::new();
        assert!(factory.list_templates().len() >= 4);
    }

    #[test]
    fn test_list_templates() {
        let factory = AgentFactory::new();
        let templates = factory.list_templates();

        let template_ids: Vec<_> = templates.iter().map(|t| t.id.as_str()).collect();
        assert!(template_ids.contains(&"code_agent"));
        assert!(template_ids.contains(&"office_agent"));
        assert!(template_ids.contains(&"data_agent"));
        assert!(template_ids.contains(&"ops_agent"));
    }
}
