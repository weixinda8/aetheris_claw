use crate::core::planner::ExecutionPlan;
use crate::utils::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub priority: u8,
    pub metadata: serde_json::Value,
    pub tags: Vec<String>,
    pub result: Option<String>,
}

impl Task {
    pub fn new(description: String, priority: u8) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: crate::utils::crypto::generate_id(),
            title: description.clone(),
            description,
            status: TaskStatus::Pending,
            created_at: now,
            updated_at: now,
            priority,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            tags: Vec::new(),
            result: None,
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn mark_running(&mut self) {
        self.status = TaskStatus::Running;
        self.updated_at = chrono::Utc::now();
    }

    pub fn mark_completed(&mut self) {
        self.status = TaskStatus::Completed;
        self.updated_at = chrono::Utc::now();
    }

    pub fn mark_failed(&mut self) {
        self.status = TaskStatus::Failed;
        self.updated_at = chrono::Utc::now();
    }

    pub fn mark_paused(&mut self) {
        self.status = TaskStatus::Paused;
        self.updated_at = chrono::Utc::now();
    }
}

#[async_trait::async_trait]
pub trait TaskExecutor: Send + Sync {
    async fn execute(&self, task: Task) -> Result<Task>;
    fn can_execute(&self, task: &Task) -> bool;
}

impl dyn TaskExecutor {
    pub fn from_box<T: TaskExecutor + 'static>(executor: T) -> Box<Self> {
        Box::new(executor) as Box<Self>
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub context_id: String,
    pub current_task_id: Option<String>,
    pub execution_plan: Option<ExecutionPlan>,
    pub current_step: u32,
    pub total_steps: u32,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub last_updated_at: chrono::DateTime<chrono::Utc>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            context_id: uuid::Uuid::new_v4().to_string(),
            current_task_id: None,
            execution_plan: None,
            current_step: 0,
            total_steps: 0,
            started_at: now,
            last_updated_at: now,
        }
    }

    pub fn with_execution_plan(mut self, plan: ExecutionPlan) -> Self {
        self.total_steps = plan.nodes.len() as u32;
        self.execution_plan = Some(plan);
        self
    }

    pub fn update_current_task(&mut self, task_id: String, step: u32) {
        self.current_task_id = Some(task_id);
        self.current_step = step;
        self.last_updated_at = chrono::Utc::now();
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}
