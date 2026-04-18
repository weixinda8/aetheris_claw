use crate::core::Task;
use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub snapshot_id: String,
    pub task_id: String,
    pub state: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub sequence_number: u64,
    pub metadata: HashMap<String, String>,
}

impl Snapshot {
    pub fn new(task_id: String, state: serde_json::Value, sequence_number: u64) -> Self {
        Self {
            snapshot_id: uuid::Uuid::new_v4().to_string(),
            task_id,
            state,
            timestamp: chrono::Utc::now(),
            sequence_number,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    pub task_id: String,
    pub status: ExecutionStatus,
    pub current_step: u32,
    pub total_steps: u32,
    pub last_snapshot_id: Option<String>,
    pub error: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl ExecutionState {
    pub fn new(task_id: String, total_steps: u32, max_retries: u32) -> Self {
        Self {
            task_id,
            status: ExecutionStatus::Pending,
            current_step: 0,
            total_steps,
            last_snapshot_id: None,
            error: None,
            start_time: None,
            end_time: None,
            retry_count: 0,
            max_retries,
        }
    }

    pub fn is_completed(&self) -> bool {
        self.status == ExecutionStatus::Completed || self.status == ExecutionStatus::RolledBack
    }

    pub fn can_retry(&self) -> bool {
        self.status == ExecutionStatus::Failed && self.retry_count < self.max_retries
    }

    pub fn mark_running(&mut self) {
        self.status = ExecutionStatus::Running;
        if self.start_time.is_none() {
            self.start_time = Some(chrono::Utc::now());
        }
    }

    pub fn mark_paused(&mut self) {
        self.status = ExecutionStatus::Paused;
    }

    pub fn mark_completed(&mut self) {
        self.status = ExecutionStatus::Completed;
        self.end_time = Some(chrono::Utc::now());
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = ExecutionStatus::Failed;
        self.error = Some(error);
        self.retry_count += 1;
        self.end_time = Some(chrono::Utc::now());
    }

    pub fn mark_rolled_back(&mut self) {
        self.status = ExecutionStatus::RolledBack;
        self.end_time = Some(chrono::Utc::now());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyKey {
    pub key: String,
    pub task_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub result: Option<serde_json::Value>,
}

impl IdempotencyKey {
    pub fn new(key: String, task_id: String, ttl_seconds: u64) -> Self {
        let now = chrono::Utc::now();
        Self {
            key,
            task_id,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(ttl_seconds as i64),
            result: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }

    pub fn set_result(&mut self, result: serde_json::Value) {
        self.result = Some(result);
    }
}

pub struct AtomicExecutor {
    snapshots: Arc<DashMap<String, Vec<Snapshot>>>,
    execution_states: Arc<DashMap<String, ExecutionState>>,
    idempotency_keys: Arc<DashMap<String, IdempotencyKey>>,
    max_snapshots_per_task: usize,
    default_idempotency_ttl_seconds: u64,
}

impl AtomicExecutor {
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(DashMap::new()),
            execution_states: Arc::new(DashMap::new()),
            idempotency_keys: Arc::new(DashMap::new()),
            max_snapshots_per_task: 10,
            default_idempotency_ttl_seconds: 3600,
        }
    }

    pub fn with_max_snapshots(mut self, max: usize) -> Self {
        self.max_snapshots_per_task = max;
        self
    }

    pub fn with_idempotency_ttl(mut self, ttl_seconds: u64) -> Self {
        self.default_idempotency_ttl_seconds = ttl_seconds;
        self
    }

    pub fn start_execution(&self, task: &Task, total_steps: u32, max_retries: u32) -> Result<()> {
        info!("Starting execution for task: {}", task.id);

        let mut state = ExecutionState::new(task.id.clone(), total_steps, max_retries);
        state.mark_running();

        self.execution_states.insert(task.id.clone(), state);
        Ok(())
    }

    pub fn get_execution_state(&self, task_id: &str) -> Option<ExecutionState> {
        self.execution_states.get(task_id).map(|s| s.clone())
    }

    pub async fn create_snapshot(&self, task: &Task) -> Result<Snapshot> {
        info!("Creating snapshot for task: {}", task.id);

        let sequence_number = self.get_next_sequence_number(task.id.as_str());
        let state = serde_json::to_value(task)?;
        let snapshot = Snapshot::new(task.id.clone(), state, sequence_number);

        let mut task_snapshots = self
            .snapshots
            .entry(task.id.clone())
            .or_default();

        task_snapshots.push(snapshot.clone());

        while task_snapshots.len() > self.max_snapshots_per_task {
            task_snapshots.remove(0);
        }

        if let Some(mut state) = self.execution_states.get_mut(task.id.as_str()) {
            state.last_snapshot_id = Some(snapshot.snapshot_id.clone());
        }

        debug!("Snapshot created: {}", snapshot.snapshot_id);
        Ok(snapshot)
    }

    fn get_next_sequence_number(&self, task_id: &str) -> u64 {
        self.snapshots
            .get(task_id)
            .map(|snapshots| snapshots.last().map(|s| s.sequence_number + 1).unwrap_or(1))
            .unwrap_or(1)
    }

    pub fn get_snapshots(&self, task_id: &str) -> Vec<Snapshot> {
        self.snapshots
            .get(task_id)
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    pub fn get_latest_snapshot(&self, task_id: &str) -> Option<Snapshot> {
        self.snapshots
            .get(task_id)
            .and_then(|snapshots| snapshots.last().cloned())
    }

    pub fn get_snapshot_by_id(&self, task_id: &str, snapshot_id: &str) -> Option<Snapshot> {
        self.snapshots.get(task_id).and_then(|snapshots| {
            snapshots
                .iter()
                .find(|s| s.snapshot_id == snapshot_id)
                .cloned()
        })
    }

    pub async fn rollback(&self, task_id: &str, snapshot_id: Option<String>) -> Result<Task> {
        info!("Rolling back task: {}", task_id);

        let snapshot = if let Some(id) = snapshot_id {
            self.get_snapshot_by_id(task_id, &id)
                .ok_or_else(|| AetherisError::Runtime(format!("Snapshot not found: {}", id)))?
        } else {
            self.get_latest_snapshot(task_id).ok_or_else(|| {
                AetherisError::Runtime("No snapshots available for rollback".to_string())
            })?
        };

        let task: Task = serde_json::from_value(snapshot.state)?;

        if let Some(mut state) = self.execution_states.get_mut(task_id) {
            state.mark_rolled_back();
        }

        info!(
            "Successfully rolled back task to snapshot: {}",
            snapshot.snapshot_id
        );
        Ok(task)
    }

    pub async fn pause_execution(&self, task_id: &str) -> Result<()> {
        info!("Pausing execution for task: {}", task_id);

        if let Some(mut state) = self.execution_states.get_mut(task_id) {
            if state.status == ExecutionStatus::Running {
                state.mark_paused();
                debug!("Execution paused for task: {}", task_id);
                Ok(())
            } else {
                Err(AetherisError::Runtime(format!(
                    "Cannot pause task in state: {:?}",
                    state.status
                )))
            }
        } else {
            Err(AetherisError::Runtime("Task not found".to_string()))
        }
    }

    pub async fn resume_execution(&self, task_id: &str) -> Result<()> {
        info!("Resuming execution for task: {}", task_id);

        if let Some(mut state) = self.execution_states.get_mut(task_id) {
            if state.status == ExecutionStatus::Paused {
                state.mark_running();
                debug!("Execution resumed for task: {}", task_id);
                Ok(())
            } else {
                Err(AetherisError::Runtime(format!(
                    "Cannot resume task in state: {:?}",
                    state.status
                )))
            }
        } else {
            Err(AetherisError::Runtime("Task not found".to_string()))
        }
    }

    pub async fn complete_execution(&self, task_id: &str) -> Result<()> {
        info!("Completing execution for task: {}", task_id);

        if let Some(mut state) = self.execution_states.get_mut(task_id) {
            state.mark_completed();
            debug!("Execution completed for task: {}", task_id);
            Ok(())
        } else {
            Err(AetherisError::Runtime("Task not found".to_string()))
        }
    }

    pub async fn fail_execution(&self, task_id: &str, error: String) -> Result<()> {
        info!("Failing execution for task: {} - Error: {}", task_id, error);

        if let Some(mut state) = self.execution_states.get_mut(task_id) {
            state.mark_failed(error);
            debug!("Execution failed for task: {}", task_id);
            Ok(())
        } else {
            Err(AetherisError::Runtime("Task not found".to_string()))
        }
    }

    pub async fn check_idempotency(&self, key: &str) -> Result<Option<serde_json::Value>> {
        if let Some(idempotency_key) = self.idempotency_keys.get(key) {
            if idempotency_key.is_expired() {
                self.idempotency_keys.remove(key);
                Ok(None)
            } else {
                Ok(idempotency_key.result.clone())
            }
        } else {
            Ok(None)
        }
    }

    pub async fn store_idempotency_result(
        &self,
        key: &str,
        task_id: &str,
        result: serde_json::Value,
    ) -> Result<()> {
        let mut idempotency_key = IdempotencyKey::new(
            key.to_string(),
            task_id.to_string(),
            self.default_idempotency_ttl_seconds,
        );
        idempotency_key.set_result(result);

        self.idempotency_keys
            .insert(key.to_string(), idempotency_key);
        Ok(())
    }

    pub async fn execute_idempotent(
        &self,
        task: Task,
        idempotency_key: Option<String>,
        execute_fn: impl FnOnce(Task) -> Result<Task>,
    ) -> Result<Task> {
        if let Some(key) = &idempotency_key {
            if let Some(cached_result) = self.check_idempotency(key).await? {
                info!("Returning cached result for idempotency key: {}", key);
                let task: Task = serde_json::from_value(cached_result)?;
                return Ok(task);
            }
        }

        let result = execute_fn(task.clone())?;

        if let Some(key) = idempotency_key {
            let result_value = serde_json::to_value(&result)?;
            self.store_idempotency_result(&key, &task.id, result_value)
                .await?;
        }

        Ok(result)
    }

    pub async fn update_step(&self, task_id: &str, step: u32) -> Result<()> {
        if let Some(mut state) = self.execution_states.get_mut(task_id) {
            state.current_step = step;
            Ok(())
        } else {
            Err(AetherisError::Runtime("Task not found".to_string()))
        }
    }

    pub fn cleanup_expired_idempotency_keys(&self) {
        let expired_keys: Vec<String> = self
            .idempotency_keys
            .iter()
            .filter(|entry| entry.is_expired())
            .map(|entry| entry.key().clone())
            .collect();

        for key in expired_keys {
            self.idempotency_keys.remove(&key);
            debug!("Cleaned up expired idempotency key: {}", key);
        }
    }
}

impl Default for AtomicExecutor {
    fn default() -> Self {
        Self::new()
    }
}
