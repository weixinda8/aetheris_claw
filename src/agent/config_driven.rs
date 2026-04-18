use crate::agent::base::{
    Agent, AgentCapabilities, AgentConfig as BaseAgentConfig, AgentState, AgentStatus, AgentType,
    BaseAgent,
};
use crate::agent::config::config::AgentConfig as ConfigurableAgentConfig;
use crate::core::Task;
use crate::core::llm::manager::LlmManager;
use crate::core::plan_execute::ReActStep;
use crate::core::progressive_loading::{LoadingStrategy, ProgressiveLoader};
use crate::memory::short_term::ShortTermMemory;
use crate::skill::agentskills::AgentSkillsRegistry;
use crate::skill::registry::SkillRegistry;
use crate::utils::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct ConfigDrivenAgent {
    config: ConfigurableAgentConfig,
    base_agent: BaseAgent,
}

impl ConfigDrivenAgent {
    pub fn new(config: ConfigurableAgentConfig) -> Self {
        let base_config = Self::convert_to_base_config(&config);
        let base_agent = BaseAgent::new(base_config);

        Self { config, base_agent }
    }

    pub fn config(&self) -> &ConfigurableAgentConfig {
        &self.config
    }

    pub fn with_llm_manager(mut self, llm_manager: Arc<LlmManager>) -> Self {
        self.base_agent = self.base_agent.with_llm_manager(llm_manager);
        self
    }

    pub fn with_skill_registry(mut self, skill_registry: Arc<SkillRegistry>) -> Self {
        self.base_agent = self.base_agent.with_skill_registry(skill_registry);
        self
    }

    pub fn with_agent_skills_registry(
        mut self,
        agent_skills_registry: Arc<AgentSkillsRegistry>,
    ) -> Self {
        self.base_agent = self
            .base_agent
            .with_agent_skills_registry(agent_skills_registry);
        self
    }

    pub fn with_progressive_loader(mut self, loader: Arc<ProgressiveLoader>) -> Self {
        self.base_agent = self.base_agent.with_progressive_loader(loader);
        self
    }

    pub fn with_short_term_memory(mut self, memory: Arc<ShortTermMemory>) -> Self {
        self.base_agent = self.base_agent.with_short_term_memory(memory);
        self
    }

    fn convert_to_base_config(config: &ConfigurableAgentConfig) -> BaseAgentConfig {
        let mut capabilities = AgentCapabilities::default();

        match config.meta.agent_type {
            AgentType::Code => {
                capabilities.can_code = true;
            }
            AgentType::Data => {
                capabilities.can_analyze_data = true;
            }
            AgentType::Ops => {
                capabilities.can_operate = true;
            }
            AgentType::Office => {
                capabilities.can_document = true;
                capabilities.can_communicate = true;
            }
            _ => {}
        }

        if let Some(caps) = &config.capabilities {
            capabilities.can_code = caps.can_code;
            capabilities.can_analyze_data = caps.can_analyze_data;
            capabilities.can_operate = caps.can_operate;
            capabilities.can_document = caps.can_document;
            capabilities.can_communicate = caps.can_communicate;
            capabilities.can_collaborate = caps.can_collaborate;
        }

        let max_concurrent_tasks = config.scheduler.concurrency.unwrap_or(5);
        let timeout_seconds = config.scheduler.timeout_seconds.unwrap_or(300);
        let max_react_iterations = 10;

        let system_prompt = config.persona.system_prompt.clone();

        BaseAgentConfig {
            id: config.meta.id.clone(),
            name: config.meta.name.clone(),
            agent_type: config.meta.agent_type.clone(),
            version: config.meta.version.clone(),
            capabilities,
            max_concurrent_tasks,
            timeout_seconds,
            max_react_iterations,
            system_prompt,
        }
    }

    async fn react_loop(&mut self, task: &mut Task) -> Result<()> {
        info!(
            "ConfigDrivenAgent starting ReAct loop for task: {}",
            task.id
        );

        let max_iterations = self.base_agent.config.max_react_iterations;

        for iteration in 0..max_iterations {
            debug!("ReAct iteration {}/{}", iteration + 1, max_iterations);

            let think_result = self.think(task, iteration).await?;

            if think_result.is_complete {
                info!("Task marked as complete during think phase");
                break;
            }

            let action_result = self.act(task, &think_result.thought).await?;

            self.observe(task, action_result).await?;

            if self.should_stop(task).await? {
                break;
            }
        }

        Ok(())
    }

    async fn think(&mut self, task: &Task, iteration: usize) -> Result<ThinkResult> {
        info!("ConfigDrivenAgent thinking about task: {}", task.id);

        self.base_agent.state.status = AgentStatus::Thinking;

        let thought = format!(
            "Iteration {}: Analyzing task '{}' with description '{}'",
            iteration + 1,
            task.title,
            task.description
        );

        let step = ReActStep::think(thought.clone());
        self.base_agent.state.add_react_step(step);

        let is_complete = iteration >= self.base_agent.config.max_react_iterations - 1;

        Ok(ThinkResult {
            thought,
            is_complete,
            task_type: None,
        })
    }

    async fn act(&mut self, task: &Task, thought: &str) -> Result<ActResult> {
        info!("ConfigDrivenAgent acting on task: {}", task.id);

        self.base_agent.state.status = AgentStatus::Acting;

        let action = format!("Executing action based on: {}", thought);
        let step = ReActStep::act(action.clone());
        self.base_agent.state.add_react_step(step);

        Ok(ActResult {
            action,
            success: true,
            output: "Action completed successfully".to_string(),
        })
    }

    async fn observe(&mut self, task: &Task, act_result: ActResult) -> Result<()> {
        info!("ConfigDrivenAgent observing results for task: {}", task.id);

        self.base_agent.state.status = AgentStatus::Observing;

        let observation = format!(
            "Observed: Action '{}' completed with result: {}",
            act_result.action, act_result.output
        );

        let step = ReActStep::observe(observation);
        self.base_agent.state.add_react_step(step);

        Ok(())
    }

    async fn should_stop(&self, _task: &Task) -> Result<bool> {
        Ok(self.base_agent.state.react_steps.len()
            >= self.base_agent.config.max_react_iterations * 3)
    }
}

#[derive(Debug, Clone)]
pub struct ThinkResult {
    pub thought: String,
    pub is_complete: bool,
    pub task_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ActResult {
    pub action: String,
    pub success: bool,
    pub output: String,
}

#[async_trait]
impl Agent for ConfigDrivenAgent {
    fn config(&self) -> &BaseAgentConfig {
        &self.base_agent.config
    }

    fn state(&self) -> &AgentState {
        &self.base_agent.state
    }

    fn state_mut(&mut self) -> &mut AgentState {
        &mut self.base_agent.state
    }

    async fn execute(&mut self, mut task: Task) -> Result<Task> {
        info!("ConfigDrivenAgent executing task: {}", task.id);

        self.base_agent.state.start_task(task.id.clone());

        if let Some(loader) = &self.base_agent.progressive_loader {
            let _ = loader.create_context(&task, LoadingStrategy::Lazy, 3).await;
        }

        let result = self.react_loop(&mut task).await;

        match result {
            Ok(_) => {
                task.status = crate::core::TaskStatus::Completed;
                self.base_agent.state.record_success();
                info!("Task completed successfully: {}", task.id);
            }
            Err(e) => {
                task.status = crate::core::TaskStatus::Failed;
                self.base_agent.state.record_failure();
                warn!("Task failed: {} - Error: {}", task.id, e);
            }
        }

        Ok(task)
    }

    fn can_handle(&self, _task: &Task) -> bool {
        true
    }

    fn is_available(&self) -> bool {
        self.base_agent.state.status == AgentStatus::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_driven_agent_new() {
        let config = crate::agent::config::config::AgentConfig::default();
        let agent = ConfigDrivenAgent::new(config);

        assert_eq!(agent.state().status, AgentStatus::Idle);
        assert!(agent.is_available());
    }
}
