use crate::workflow::{Workflow, WorkflowStatus};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WorkflowEventType {
    WorkflowCreated,
    WorkflowUpdated,
    WorkflowDeleted,
    WorkflowStarted,
    WorkflowCompleted,
    WorkflowFailed,
    WorkflowPaused,
    WorkflowResumed,
    WorkflowCancelled,
    NodeStarted,
    NodeCompleted,
    NodeFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    pub event_type: WorkflowEventType,
    pub workflow_id: String,
    pub node_id: Option<String>,
    pub status: Option<WorkflowStatus>,
    pub message: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<serde_json::Value>,
}

impl WorkflowEvent {
    pub fn new(
        event_type: WorkflowEventType,
        workflow_id: String,
        node_id: Option<String>,
        status: Option<WorkflowStatus>,
        message: Option<String>,
    ) -> Self {
        Self {
            event_type,
            workflow_id,
            node_id,
            status,
            message,
            timestamp: chrono::Utc::now(),
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

type EventCallback = Box<dyn Fn(&WorkflowEvent) + Send + Sync + 'static>;

pub struct WorkflowEventBus {
    sender: broadcast::Sender<WorkflowEvent>,
    subscribers: Arc<DashMap<WorkflowEventType, Vec<Arc<EventCallback>>>>,
}

impl WorkflowEventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            sender,
            subscribers: Arc::new(DashMap::new()),
        }
    }

    pub fn subscribe<F>(&self, event_type: WorkflowEventType, callback: F)
    where
        F: Fn(&WorkflowEvent) + Send + Sync + 'static,
    {
        let callback = Arc::new(Box::new(callback) as EventCallback);
        self.subscribers
            .entry(event_type)
            .or_default()
            .push(callback);
    }

    pub fn subscribe_all<F>(&self, callback: F)
    where
        F: Fn(&WorkflowEvent) + Send + Sync + 'static,
    {
        let callback = Arc::new(Box::new(callback) as EventCallback);

        for event_type in &[
            WorkflowEventType::WorkflowCreated,
            WorkflowEventType::WorkflowUpdated,
            WorkflowEventType::WorkflowDeleted,
            WorkflowEventType::WorkflowStarted,
            WorkflowEventType::WorkflowCompleted,
            WorkflowEventType::WorkflowFailed,
            WorkflowEventType::WorkflowPaused,
            WorkflowEventType::WorkflowResumed,
            WorkflowEventType::WorkflowCancelled,
            WorkflowEventType::NodeStarted,
            WorkflowEventType::NodeCompleted,
            WorkflowEventType::NodeFailed,
        ] {
            self.subscribers
                .entry(event_type.clone())
                .or_default()
                .push(callback.clone());
        }
    }

    pub fn publish(&self, event: WorkflowEvent) {
        let _ = self.sender.send(event.clone());

        if let Some(callbacks) = self.subscribers.get(&event.event_type) {
            for callback in callbacks.iter() {
                callback(&event);
            }
        }
    }

    pub fn subscribe_receiver(&self) -> broadcast::Receiver<WorkflowEvent> {
        self.sender.subscribe()
    }
}

impl Default for WorkflowEventBus {
    fn default() -> Self {
        Self::new()
    }
}
