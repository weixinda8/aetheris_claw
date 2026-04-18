use crate::core::Task;
use crate::core::llm::manager::LlmManager;
use crate::core::plan_execute::ReActStep;
use crate::core::progressive_loading::{LoadingStrategy, ProgressiveLoader};
use crate::memory::short_term::ShortTermMemory;
use crate::skill::agentskills::AgentSkillsRegistry;
use crate::skill::registry::SkillRegistry;
use crate::utils::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentType {
    Code,
    Data,
    Ops,
    Office,
    Industrial,
    Compliance,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    pub can_code: bool,
    pub can_analyze_data: bool,
    pub can_operate: bool,
    pub can_document: bool,
    pub can_communicate: bool,
    pub can_collaborate: bool,
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self {
            can_code: false,
            can_analyze_data: false,
            can_operate: false,
            can_document: false,
            can_communicate: true,
            can_collaborate: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub agent_type: AgentType,
    pub version: String,
    pub capabilities: AgentCapabilities,
    pub max_concurrent_tasks: usize,
    pub timeout_seconds: u64,
    pub max_react_iterations: usize,
    pub system_prompt: Option<String>,
}

impl AgentConfig {
    pub fn new(id: String, name: String, agent_type: AgentType) -> Self {
        Self {
            id,
            name,
            agent_type,
            version: "1.0.0".to_string(),
            capabilities: AgentCapabilities::default(),
            max_concurrent_tasks: 5,
            timeout_seconds: 300,
            max_react_iterations: 10,
            system_prompt: None,
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self::new(
            "default-agent".to_string(),
            "Default Agent".to_string(),
            AgentType::Generic,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AgentStatus {
    #[default]
    Idle,
    Busy,
    Paused,
    Error,
    Thinking,
    Acting,
    Observing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub agent_id: String,
    pub status: AgentStatus,
    pub current_task_id: Option<String>,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub last_active_at: chrono::DateTime<chrono::Utc>,
    pub registered_at: chrono::DateTime<chrono::Utc>,
    pub react_steps: Vec<ReActStep>,
}

impl AgentState {
    pub fn new(agent_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            agent_id,
            status: AgentStatus::Idle,
            current_task_id: None,
            completed_tasks: 0,
            failed_tasks: 0,
            last_active_at: now,
            registered_at: now,
            react_steps: Vec::new(),
        }
    }

    pub fn record_success(&mut self) {
        self.completed_tasks += 1;
        self.status = AgentStatus::Idle;
        self.current_task_id = None;
        self.last_active_at = chrono::Utc::now();
    }

    pub fn record_failure(&mut self) {
        self.failed_tasks += 1;
        self.status = AgentStatus::Idle;
        self.current_task_id = None;
        self.last_active_at = chrono::Utc::now();
    }

    pub fn start_task(&mut self, task_id: String) {
        self.status = AgentStatus::Busy;
        self.current_task_id = Some(task_id);
        self.last_active_at = chrono::Utc::now();
        self.react_steps.clear();
    }

    pub fn add_react_step(&mut self, step: ReActStep) {
        self.react_steps.push(step);
        self.last_active_at = chrono::Utc::now();
    }
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new("default-agent".to_string())
    }
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn config(&self) -> &AgentConfig;
    fn state(&self) -> &AgentState;
    fn state_mut(&mut self) -> &mut AgentState;
    async fn execute(&mut self, task: Task) -> Result<Task>;
    fn can_handle(&self, task: &Task) -> bool;
    fn is_available(&self) -> bool;
}

impl dyn Agent + Send + Sync {
    pub fn from_arc<T: Agent + Send + Sync + 'static>(agent: T) -> Arc<Self> {
        Arc::new(agent) as Arc<Self>
    }
}

pub struct BaseAgent {
    pub config: AgentConfig,
    pub state: AgentState,
    pub llm_manager: Option<Arc<LlmManager>>,
    pub skill_registry: Option<Arc<SkillRegistry>>,
    pub agent_skills_registry: Option<Arc<AgentSkillsRegistry>>,
    pub short_term_memory: Arc<ShortTermMemory>,
    pub progressive_loader: Option<Arc<ProgressiveLoader>>,
}

impl BaseAgent {
    pub fn new(config: AgentConfig) -> Self {
        let state = AgentState::new(config.id.clone());
        Self {
            config,
            state,
            llm_manager: None,
            skill_registry: None,
            agent_skills_registry: None,
            short_term_memory: Arc::new(ShortTermMemory::new()),
            progressive_loader: None,
        }
    }

    pub fn new_arc(config: AgentConfig) -> Arc<dyn Agent + Send + Sync> {
        Arc::new(Self::new(config)) as Arc<dyn Agent + Send + Sync>
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

    pub fn with_progressive_loader(mut self, loader: Arc<ProgressiveLoader>) -> Self {
        self.progressive_loader = Some(loader);
        self
    }

    pub fn with_short_term_memory(mut self, memory: Arc<ShortTermMemory>) -> Self {
        self.short_term_memory = memory;
        self
    }

    async fn react_loop(&mut self, task: &mut Task) -> Result<()> {
        info!("Starting ReAct loop for task: {}", task.id);

        let max_iterations = self.config.max_react_iterations;

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
        info!("Agent thinking about task: {}", task.id);

        self.state.status = AgentStatus::Thinking;

        let thought = format!(
            "Iteration {}: Analyzing task '{}' with description '{}'",
            iteration + 1,
            task.title,
            task.description
        );

        let step = ReActStep::think(thought.clone());
        self.state.add_react_step(step);

        let is_complete = iteration >= self.config.max_react_iterations - 1;

        Ok(ThinkResult {
            thought,
            is_complete,
            task_type: None,
        })
    }

    async fn act(&mut self, task: &Task, thought: &str) -> Result<ActResult> {
        info!("Agent acting on task: {}", task.id);

        self.state.status = AgentStatus::Acting;

        let action = format!("Executing action based on: {}", thought);
        let step = ReActStep::act(action.clone());
        self.state.add_react_step(step);

        Ok(ActResult {
            action,
            success: true,
            output: "Action completed successfully".to_string(),
        })
    }

    async fn observe(&mut self, task: &Task, act_result: ActResult) -> Result<()> {
        info!("Agent observing results for task: {}", task.id);

        self.state.status = AgentStatus::Observing;

        let observation = format!(
            "Observed: Action '{}' completed with result: {}",
            act_result.action, act_result.output
        );

        let step = ReActStep::observe(observation);
        self.state.add_react_step(step);

        Ok(())
    }

    async fn should_stop(&self, _task: &Task) -> Result<bool> {
        Ok(self.state.react_steps.len() >= self.config.max_react_iterations * 3)
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
impl Agent for BaseAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    fn state(&self) -> &AgentState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut AgentState {
        &mut self.state
    }

    async fn execute(&mut self, mut task: Task) -> Result<Task> {
        info!("BaseAgent executing task: {}", task.id);

        self.state.start_task(task.id.clone());

        if let Some(loader) = &self.progressive_loader {
            let _ = loader.create_context(&task, LoadingStrategy::Lazy, 3).await;
        }

        let result = self.react_loop(&mut task).await;

        match result {
            Ok(_) => {
                task.status = crate::core::TaskStatus::Completed;
                self.state.record_success();
                info!("Task completed successfully: {}", task.id);
            }
            Err(e) => {
                task.status = crate::core::TaskStatus::Failed;
                self.state.record_failure();
                warn!("Task failed: {} - Error: {}", task.id, e);
            }
        }

        Ok(task)
    }

    fn can_handle(&self, _task: &Task) -> bool {
        true
    }

    fn is_available(&self) -> bool {
        self.state.status == AgentStatus::Idle
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub message_id: String,
    pub from_agent_id: String,
    pub to_agent_id: String,
    pub message_type: String,
    pub payload: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl AgentMessage {
    pub fn new(from: String, to: String, msg_type: String, payload: String) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            from_agent_id: from,
            to_agent_id: to,
            message_type: msg_type,
            payload,
            timestamp: chrono::Utc::now(),
        }
    }
}

pub struct AgentRegistry {
    agents: DashMap<String, Arc<dyn Agent + Send + Sync>>,
    message_queue: DashMap<String, Vec<AgentMessage>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: DashMap::new(),
            message_queue: DashMap::new(),
        }
    }

    pub fn register_agent(&self, agent: Arc<dyn Agent + Send + Sync>) -> Result<()> {
        let agent_id = agent.config().id.clone();
        info!("Registering agent: {}", agent_id);
        self.agents.insert(agent_id.clone(), agent);
        self.message_queue.insert(agent_id, Vec::new());
        Ok(())
    }

    pub fn unregister_agent(&self, agent_id: &str) -> Result<()> {
        info!("Unregistering agent: {}", agent_id);
        self.agents.remove(agent_id);
        self.message_queue.remove(agent_id);
        Ok(())
    }

    pub fn get_agent(&self, agent_id: &str) -> Option<Arc<dyn Agent + Send + Sync>> {
        self.agents.get(agent_id).map(|a| a.clone())
    }

    pub fn get_agents_by_type(&self, agent_type: &AgentType) -> Vec<Arc<dyn Agent + Send + Sync>> {
        self.agents
            .iter()
            .filter(|entry| entry.value().config().agent_type == *agent_type)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_available_agents(&self) -> Vec<Arc<dyn Agent + Send + Sync>> {
        self.agents
            .iter()
            .filter(|entry| entry.value().is_available())
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn find_best_agent_for_task(&self, task: &Task) -> Option<Arc<dyn Agent + Send + Sync>> {
        let available_agents = self.get_available_agents();
        available_agents
            .into_iter()
            .filter(|agent| agent.can_handle(task))
            .max_by_key(|agent| {
                let config = agent.config();
                let mut score = 0;
                if config.capabilities.can_code {
                    score += 10;
                }
                if config.capabilities.can_analyze_data {
                    score += 10;
                }
                if config.capabilities.can_operate {
                    score += 10;
                }
                if config.capabilities.can_document {
                    score += 5;
                }
                score
            })
    }

    pub fn send_message(&self, message: AgentMessage) -> Result<()> {
        info!(
            "Sending message from {} to {}: {}",
            message.from_agent_id, message.to_agent_id, message.message_type
        );
        if let Some(mut queue) = self.message_queue.get_mut(&message.to_agent_id) {
            queue.push(message);
        }
        Ok(())
    }

    pub fn get_messages(&self, agent_id: &str) -> Vec<AgentMessage> {
        self.message_queue
            .get(agent_id)
            .map(|q| q.clone())
            .unwrap_or_default()
    }

    pub fn clear_messages(&self, agent_id: &str) {
        if let Some(mut queue) = self.message_queue.get_mut(agent_id) {
            queue.clear();
        }
    }

    pub fn list_all_agents(&self) -> Vec<AgentConfig> {
        self.agents
            .iter()
            .map(|entry| entry.value().config().clone())
            .collect()
    }

    pub fn get_agent_states(&self) -> Vec<AgentState> {
        self.agents
            .iter()
            .map(|entry| entry.value().state().clone())
            .collect()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
