use crate::workflow::{Workflow, WorkflowExecutionContext, WorkflowStatus};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateHistoryEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub status: WorkflowStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub workflow: Workflow,
    pub current_status: WorkflowStatus,
    pub execution_context: Option<WorkflowExecutionContext>,
    pub history: Vec<StateHistoryEntry>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl WorkflowState {
    pub fn new(workflow: Workflow) -> Self {
        let now = chrono::Utc::now();
        let initial_entry = StateHistoryEntry {
            timestamp: now,
            status: WorkflowStatus::Pending,
            message: Some("Workflow created".to_string()),
        };

        Self {
            workflow,
            current_status: WorkflowStatus::Pending,
            execution_context: None,
            history: vec![initial_entry],
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_status(&mut self, status: WorkflowStatus, message: Option<String>) {
        self.current_status = status.clone();
        self.updated_at = chrono::Utc::now();

        let entry = StateHistoryEntry {
            timestamp: self.updated_at,
            status,
            message,
        };
        self.history.push(entry);
    }
}

pub struct WorkflowStateManager {
    states: Arc<DashMap<String, WorkflowState>>,
}

impl WorkflowStateManager {
    pub fn new() -> Self {
        Self {
            states: Arc::new(DashMap::new()),
        }
    }

    pub fn create_workflow(&self, workflow: Workflow) -> String {
        let workflow_id = workflow.id.clone();
        let state = WorkflowState::new(workflow);
        self.states.insert(workflow_id.clone(), state);
        workflow_id
    }

    pub fn get_workflow(&self, workflow_id: &str) -> Option<WorkflowState> {
        self.states.get(workflow_id).map(|s| s.value().clone())
    }

    pub fn list_workflows(&self) -> Vec<WorkflowState> {
        self.states.iter().map(|s| s.value().clone()).collect()
    }

    pub fn update_workflow(&self, workflow_id: &str, workflow: Workflow) -> bool {
        if let Some(mut state) = self.states.get_mut(workflow_id) {
            state.workflow = workflow;
            state.updated_at = chrono::Utc::now();
            true
        } else {
            false
        }
    }

    pub fn delete_workflow(&self, workflow_id: &str) -> bool {
        self.states.remove(workflow_id).is_some()
    }

    pub fn update_workflow_status(
        &self,
        workflow_id: &str,
        status: WorkflowStatus,
        message: Option<String>,
    ) -> bool {
        if let Some(mut state) = self.states.get_mut(workflow_id) {
            state.update_status(status, message);
            true
        } else {
            false
        }
    }

    pub fn set_execution_context(
        &self,
        workflow_id: &str,
        context: WorkflowExecutionContext,
    ) -> bool {
        if let Some(mut state) = self.states.get_mut(workflow_id) {
            state.execution_context = Some(context);
            state.updated_at = chrono::Utc::now();
            true
        } else {
            false
        }
    }

    pub fn get_execution_context(&self, workflow_id: &str) -> Option<WorkflowExecutionContext> {
        self.states
            .get(workflow_id)
            .and_then(|s| s.execution_context.clone())
    }

    pub fn get_status(&self, workflow_id: &str) -> Option<WorkflowStatus> {
        self.states
            .get(workflow_id)
            .map(|s| s.current_status.clone())
    }
}

impl Default for WorkflowStateManager {
    fn default() -> Self {
        Self::new()
    }
}
