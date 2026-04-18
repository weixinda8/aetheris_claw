use crate::core::Task;
use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentType {
    Code,
    Data,
    Ops,
    Office,
    Industrial,
    Compliance,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent_id: String,
    pub agent_type: AgentType,
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub is_available: bool,
    pub current_task_id: Option<String>,
    pub success_rate: f64,
    pub total_tasks: u64,
}

impl AgentInfo {
    pub fn new(agent_id: String, agent_type: AgentType, name: String, description: String) -> Self {
        Self {
            agent_id,
            agent_type,
            name,
            description,
            capabilities: Vec::new(),
            is_available: true,
            current_task_id: None,
            success_rate: 1.0,
            total_tasks: 0,
        }
    }

    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn can_handle_task(&self, _task: &Task) -> bool {
        if !self.is_available {
            return false;
        }
        if self.current_task_id.is_some() {
            return false;
        }
        true
    }

    pub fn record_success(&mut self) {
        self.total_tasks += 1;
        let total = self.total_tasks as f64;
        self.success_rate = (self.success_rate * (total - 1.0) + 1.0) / total;
    }

    pub fn record_failure(&mut self) {
        self.total_tasks += 1;
        let total = self.total_tasks as f64;
        self.success_rate = (self.success_rate * (total - 1.0)) / total;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskAssignmentStatus {
    Assigned,
    InProgress,
    Completed,
    Failed,
    Reassigned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub assignment_id: String,
    pub task_id: String,
    pub agent_id: String,
    pub status: TaskAssignmentStatus,
    pub assigned_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl TaskAssignment {
    pub fn new(task_id: String, agent_id: String, max_retries: u32) -> Self {
        Self {
            assignment_id: uuid::Uuid::new_v4().to_string(),
            task_id,
            agent_id,
            status: TaskAssignmentStatus::Assigned,
            assigned_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries,
        }
    }

    pub fn mark_in_progress(&mut self) {
        self.status = TaskAssignmentStatus::InProgress;
        self.started_at = Some(chrono::Utc::now());
    }

    pub fn mark_completed(&mut self) {
        self.status = TaskAssignmentStatus::Completed;
        self.completed_at = Some(chrono::Utc::now());
    }

    pub fn mark_failed(&mut self) {
        self.status = TaskAssignmentStatus::Failed;
        self.retry_count += 1;
    }

    pub fn can_retry(&self) -> bool {
        self.status == TaskAssignmentStatus::Failed && self.retry_count < self.max_retries
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionDecision {
    pub task_id: String,
    pub exception_type: String,
    pub decision: ExceptionAction,
    pub reason: String,
    pub decided_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExceptionAction {
    Retry,
    Reassign,
    Skip,
    Fail,
    HumanIntervention,
}

type ExceptionHandlerFn = Box<dyn Fn(&Task, &str) -> Option<ExceptionDecision> + Send + Sync>;
type ExceptionHandlers = Arc<Mutex<Vec<ExceptionHandlerFn>>>;

pub struct Coordinator {
    agents: Arc<DashMap<String, AgentInfo>>,
    assignments: Arc<DashMap<String, TaskAssignment>>,
    task_agent_map: Arc<DashMap<String, String>>,
    agent_type_index: Arc<DashMap<AgentType, Vec<String>>>,
    exception_handlers: ExceptionHandlers,
    max_assignment_retries: u32,
}

impl Coordinator {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(DashMap::new()),
            assignments: Arc::new(DashMap::new()),
            task_agent_map: Arc::new(DashMap::new()),
            agent_type_index: Arc::new(DashMap::new()),
            exception_handlers: Arc::new(Mutex::new(Vec::new())),
            max_assignment_retries: 3,
        }
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_assignment_retries = retries;
        self
    }

    pub fn register_agent(&self, agent: AgentInfo) -> Result<()> {
        info!(
            "Registering agent: {} ({:?})",
            agent.agent_id, agent.agent_type
        );

        let agent_id = agent.agent_id.clone();
        let agent_type = agent.agent_type.clone();

        self.agents.insert(agent_id.clone(), agent);

        self.agent_type_index
            .entry(agent_type)
            .or_default()
            .push(agent_id);

        debug!("Agent registered successfully");
        Ok(())
    }

    pub fn unregister_agent(&self, agent_id: &str) -> Result<()> {
        info!("Unregistering agent: {}", agent_id);

        if let Some((_, agent)) = self.agents.remove(agent_id) {
            if let Some(mut agent_ids) = self.agent_type_index.get_mut(&agent.agent_type) {
                agent_ids.retain(|id| id != agent_id);
            }
        }

        Ok(())
    }

    pub fn get_agent(&self, agent_id: &str) -> Option<AgentInfo> {
        self.agents.get(agent_id).map(|a| a.value().clone())
    }

    pub fn get_agents_by_type(&self, agent_type: &AgentType) -> Vec<AgentInfo> {
        self.agent_type_index
            .get(agent_type)
            .map(|agent_ids| {
                agent_ids
                    .iter()
                    .filter_map(|id| self.agents.get(id).map(|a| a.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_available_agents(&self, task: &Task) -> Vec<AgentInfo> {
        self.agents
            .iter()
            .filter(|entry| entry.value().can_handle_task(task))
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn select_best_agent(&self, task: &Task) -> Result<AgentInfo> {
        let available_agents = self.get_available_agents(task);

        if available_agents.is_empty() {
            return Err(AetherisError::Agent(
                "No available agents for task".to_string(),
            ));
        }

        let mut best_agent = available_agents[0].clone();
        let mut best_score = best_agent.success_rate;

        for agent in &available_agents {
            let mut score = agent.success_rate;

            if !agent.capabilities.is_empty() {
                score += 0.1;
            }

            if score > best_score {
                best_score = score;
                best_agent = agent.clone();
            }
        }

        Ok(best_agent)
    }

    pub async fn assign_task(&self, task: &Task) -> Result<TaskAssignment> {
        info!("Assigning task: {}", task.id);

        let agent = self.select_best_agent(task)?;
        let agent_id_clone = agent.agent_id.clone();
        let assignment = TaskAssignment::new(
            task.id.clone(),
            agent_id_clone.clone(),
            self.max_assignment_retries,
        );

        if let Some(mut agent_mut) = self.agents.get_mut(&agent_id_clone) {
            agent_mut.is_available = false;
            agent_mut.current_task_id = Some(task.id.clone());
        }

        self.assignments
            .insert(assignment.assignment_id.clone(), assignment.clone());
        self.task_agent_map.insert(task.id.clone(), agent_id_clone);

        debug!("Task {} assigned to agent {}", task.id, agent.agent_id);
        Ok(assignment)
    }

    pub async fn start_task(&self, task_id: &str) -> Result<()> {
        info!("Starting task execution: {}", task_id);

        if let Some(_agent_id) = self.task_agent_map.get(task_id) {
            for mut entry in self.assignments.iter_mut() {
                let assignment = entry.value_mut();
                if assignment.task_id == task_id
                    && assignment.status == TaskAssignmentStatus::Assigned
                {
                    assignment.mark_in_progress();
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn complete_task(&self, task_id: &str) -> Result<()> {
        info!("Completing task: {}", task_id);

        if let Some(agent_id) = self.task_agent_map.get(task_id) {
            let agent_id_clone = agent_id.clone();
            if let Some(mut agent) = self.agents.get_mut(&agent_id_clone) {
                agent.record_success();
                agent.is_available = true;
                agent.current_task_id = None;
            }

            for mut entry in self.assignments.iter_mut() {
                let assignment = entry.value_mut();
                if assignment.task_id == task_id
                    && assignment.status == TaskAssignmentStatus::InProgress
                {
                    assignment.mark_completed();
                    break;
                }
            }

            self.task_agent_map.remove(task_id);
        }

        Ok(())
    }

    pub async fn fail_task(&self, task_id: &str, error: &str) -> Result<ExceptionDecision> {
        info!("Handling task failure: {} - Error: {}", task_id, error);

        let mut needs_retry = false;

        if let Some(agent_id) = self.task_agent_map.get(task_id) {
            let agent_id_clone = agent_id.clone();
            if let Some(mut agent) = self.agents.get_mut(&agent_id_clone) {
                agent.record_failure();
                agent.is_available = true;
                agent.current_task_id = None;
            }

            for mut entry in self.assignments.iter_mut() {
                let assignment = entry.value_mut();
                if assignment.task_id == task_id
                    && assignment.status == TaskAssignmentStatus::InProgress
                {
                    assignment.mark_failed();
                    if assignment.can_retry() {
                        needs_retry = true;
                    }
                    break;
                }
            }
        }

        let decision = if needs_retry {
            ExceptionDecision {
                task_id: task_id.to_string(),
                exception_type: "TaskFailure".to_string(),
                decision: ExceptionAction::Retry,
                reason: "Task failed, retrying".to_string(),
                decided_at: chrono::Utc::now(),
            }
        } else {
            let handlers = self.exception_handlers.lock().await;
            let mut custom_decision = None;

            if let Some(task) = self.get_task_for_failure(task_id) {
                for handler in handlers.iter() {
                    if let Some(dec) = handler(&task, error) {
                        custom_decision = Some(dec);
                        break;
                    }
                }
            }

            custom_decision.unwrap_or_else(|| ExceptionDecision {
                task_id: task_id.to_string(),
                exception_type: "TaskFailure".to_string(),
                decision: ExceptionAction::Fail,
                reason: "Max retries exceeded".to_string(),
                decided_at: chrono::Utc::now(),
            })
        };

        Ok(decision)
    }

    fn get_task_for_failure(&self, _task_id: &str) -> Option<Task> {
        None
    }

    pub fn register_exception_handler<F>(&self, handler: F)
    where
        F: Fn(&Task, &str) -> Option<ExceptionDecision> + Send + Sync + 'static,
    {
        let mut handlers = self.exception_handlers.try_lock().unwrap();
        handlers.push(Box::new(handler));
    }

    pub fn get_assignment(&self, task_id: &str) -> Option<TaskAssignment> {
        self.assignments
            .iter()
            .find(|entry| entry.value().task_id == task_id)
            .map(|entry| entry.value().clone())
    }

    pub fn get_active_assignments(&self) -> Vec<TaskAssignment> {
        self.assignments
            .iter()
            .filter(|entry| {
                matches!(
                    entry.value().status,
                    TaskAssignmentStatus::Assigned | TaskAssignmentStatus::InProgress
                )
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub async fn coordinate(&self, tasks: Vec<Task>) -> Result<Vec<Task>> {
        info!("Coordinating {} tasks", tasks.len());

        let mut results = Vec::new();

        for task in tasks {
            let _assignment = self.assign_task(&task).await?;
            self.start_task(&task.id).await?;
            self.complete_task(&task.id).await?;
            results.push(task);
        }

        Ok(results)
    }

    pub async fn handle_failure(&self, task: Task) -> Result<Task> {
        info!("Handling failure for task: {}", task.id);
        Ok(task)
    }
}

impl Default for Coordinator {
    fn default() -> Self {
        Self::new()
    }
}
