use crate::core::{Task, planner};
use crate::utils::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReActStepType {
    Think,
    Act,
    Observe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReActStep {
    pub step_id: String,
    pub step_type: ReActStepType,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub observation: Option<String>,
}

impl ReActStep {
    pub fn new(step_type: ReActStepType, content: String) -> Self {
        Self {
            step_id: uuid::Uuid::new_v4().to_string(),
            step_type,
            content,
            timestamp: chrono::Utc::now(),
            observation: None,
        }
    }

    pub fn with_observation(mut self, observation: String) -> Self {
        self.observation = Some(observation);
        self
    }

    pub fn think(content: String) -> Self {
        Self::new(ReActStepType::Think, content)
    }

    pub fn act(content: String) -> Self {
        Self::new(ReActStepType::Act, content)
    }

    pub fn observe(content: String) -> Self {
        Self::new(ReActStepType::Observe, content)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlanExecuteStatus {
    Planning,
    Executing,
    ReActing,
    Reflecting,
    Completed,
    Failed,
    Replaning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecuteState {
    pub state_id: String,
    pub original_task: Task,
    pub current_plan: Option<planner::ExecutionPlan>,
    pub current_step_index: usize,
    pub react_steps: Vec<ReActStep>,
    pub status: PlanExecuteStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_updated_at: chrono::DateTime<chrono::Utc>,
}

impl PlanExecuteState {
    pub fn new(task: Task, max_retries: u32) -> Self {
        let now = chrono::Utc::now();
        Self {
            state_id: uuid::Uuid::new_v4().to_string(),
            original_task: task,
            current_plan: None,
            current_step_index: 0,
            react_steps: Vec::new(),
            status: PlanExecuteStatus::Planning,
            retry_count: 0,
            max_retries,
            created_at: now,
            last_updated_at: now,
        }
    }

    pub fn with_plan(mut self, plan: planner::ExecutionPlan) -> Self {
        self.current_plan = Some(plan);
        self.status = PlanExecuteStatus::Executing;
        self.last_updated_at = chrono::Utc::now();
        self
    }

    pub fn add_react_step(&mut self, step: ReActStep) {
        self.react_steps.push(step);
        self.last_updated_at = chrono::Utc::now();
    }

    pub fn mark_completed(&mut self) {
        self.status = PlanExecuteStatus::Completed;
        self.last_updated_at = chrono::Utc::now();
    }

    pub fn mark_failed(&mut self) {
        self.status = PlanExecuteStatus::Failed;
        self.retry_count += 1;
        self.last_updated_at = chrono::Utc::now();
    }

    pub fn advance_step(&mut self) {
        self.current_step_index += 1;
        self.last_updated_at = chrono::Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecuteResult {
    pub success: bool,
    pub final_task: Task,
    pub state: PlanExecuteState,
    pub execution_time_seconds: f64,
    pub total_steps: usize,
    pub lessons_learned: Vec<String>,
}

pub struct PlanAndExecuteEngine {
    planner: Arc<Mutex<planner::TaskPlanner>>,
    reflector: Arc<crate::core::reflect::Reflector>,
    max_plan_retries: u32,
    max_react_iterations: usize,
}

impl PlanAndExecuteEngine {
    pub fn new() -> Self {
        Self {
            planner: Arc::new(Mutex::new(planner::TaskPlanner::new())),
            reflector: Arc::new(crate::core::reflect::Reflector::new()),
            max_plan_retries: 3,
            max_react_iterations: 10,
        }
    }

    pub fn with_planner(mut self, planner: planner::TaskPlanner) -> Self {
        self.planner = Arc::new(Mutex::new(planner));
        self
    }

    pub fn with_reflector(mut self, reflector: crate::core::reflect::Reflector) -> Self {
        self.reflector = Arc::new(reflector);
        self
    }

    pub fn with_max_plan_retries(mut self, retries: u32) -> Self {
        self.max_plan_retries = retries;
        self
    }

    pub fn with_max_react_iterations(mut self, iterations: usize) -> Self {
        self.max_react_iterations = iterations;
        self
    }

    pub async fn execute(&self, task: Task) -> Result<PlanExecuteResult> {
        info!("Starting Plan-and-Execute for task: {}", task.id);

        let start_time = chrono::Utc::now();
        let mut state = PlanExecuteState::new(task.clone(), self.max_plan_retries);
        let mut current_retry = 0;

        let (success, final_task, lessons_learned) = loop {
            match self.execute_single_attempt(&mut state, current_retry).await {
                Ok(task_result) => {
                    state.mark_completed();
                    let report = self.reflector.analyze(&task_result).await?;
                    let lessons = report.lessons_learned.clone();
                    break (true, task_result, lessons);
                }
                Err(e) => {
                    current_retry += 1;
                    warn!("Attempt {} failed: {}", current_retry, e);

                    if current_retry >= self.max_plan_retries {
                        state.mark_failed();
                        break (false, state.original_task.clone(), Vec::new());
                    }

                    let think_step = ReActStep::think(format!(
                        "Attempt {} failed, retrying. Error: {}",
                        current_retry, e
                    ));
                    state.add_react_step(think_step);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        };

        let end_time = chrono::Utc::now();
        let duration_seconds = (end_time - start_time).num_milliseconds() as f64 / 1000.0;

        Ok(PlanExecuteResult {
            success,
            final_task,
            state: state.clone(),
            execution_time_seconds: duration_seconds,
            total_steps: state.react_steps.len() + state.current_step_index,
            lessons_learned,
        })
    }

    async fn execute_single_attempt(
        &self,
        state: &mut PlanExecuteState,
        attempt: u32,
    ) -> Result<Task> {
        info!(
            "Planning phase for task: {} (attempt {})",
            state.original_task.id,
            attempt + 1
        );
        state.status = PlanExecuteStatus::Planning;

        let mut planner = self.planner.lock().await;
        let sub_tasks = planner.decompose_task_sync(&state.original_task)?;
        let plan = planner.plan(state.original_task.clone(), sub_tasks)?;
        drop(planner);

        let state_ref = PlanExecuteState::new(state.original_task.clone(), state.max_retries);
        let mut state_with_plan = state_ref.with_plan(plan);

        let think_step = ReActStep::think("Plan created, starting execution".to_string());
        state_with_plan.add_react_step(think_step);

        self.execute_plan_steps(&mut state_with_plan).await?;

        state.current_plan = state_with_plan.current_plan;
        state.current_step_index = state_with_plan.current_step_index;
        state.react_steps.extend(state_with_plan.react_steps);

        self.reflect_and_complete(state).await
    }

    async fn execute_plan_steps(&self, state: &mut PlanExecuteState) -> Result<()> {
        info!("Executing plan steps for task: {}", state.original_task.id);

        if let Some(plan) = &state.current_plan {
            let node_count = plan.nodes.len();

            while state.current_step_index < node_count {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                let act_step = ReActStep::act(format!(
                    "Executing step {}/{}",
                    state.current_step_index + 1,
                    node_count
                ));
                state.add_react_step(act_step);

                let observe_step = ReActStep::observe("Step completed successfully".to_string());
                state.add_react_step(observe_step);

                state.advance_step();
            }
        }

        Ok(())
    }

    async fn reflect_and_complete(&self, state: &mut PlanExecuteState) -> Result<Task> {
        info!("Reflection phase for task: {}", state.original_task.id);
        state.status = PlanExecuteStatus::Reflecting;

        let think_step = ReActStep::think("Reflecting on execution...".to_string());
        state.add_react_step(think_step);

        let mut final_task = state.original_task.clone();
        final_task.mark_completed();

        Ok(final_task)
    }
}

impl Default for PlanAndExecuteEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ReActEngine {
    max_iterations: usize,
    reflector: Arc<crate::core::reflect::Reflector>,
}

impl ReActEngine {
    pub fn new() -> Self {
        Self {
            max_iterations: 10,
            reflector: Arc::new(crate::core::reflect::Reflector::new()),
        }
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_reflector(mut self, reflector: crate::core::reflect::Reflector) -> Self {
        self.reflector = Arc::new(reflector);
        self
    }

    pub async fn run(&self, task: Task) -> Result<(Task, Vec<ReActStep>)> {
        info!("Starting ReAct reasoning for task: {}", task.id);

        let mut steps = Vec::new();
        let mut current_task = task.clone();

        let think_step = ReActStep::think("Starting task execution...".to_string());
        steps.push(think_step);

        if let Some(iteration) = (0..self.max_iterations).next() {
            let act_step = ReActStep::act(format!("Iteration {}: Executing task", iteration + 1));
            steps.push(act_step);

            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            let observe_step = ReActStep::observe("Execution completed successfully".to_string());
            steps.push(observe_step);

            let complete_think =
                ReActStep::think("Task completed, no further actions needed".to_string());
            steps.push(complete_think);

            current_task.mark_completed();
        }

        Ok((current_task, steps))
    }
}

impl Default for ReActEngine {
    fn default() -> Self {
        Self::new()
    }
}
