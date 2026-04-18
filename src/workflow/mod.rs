pub mod dag;
pub mod dsl;
pub mod events;
pub mod execution;
pub mod retry;
pub mod state;
pub mod timeout;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Paused,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub status: WorkflowStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<serde_json::Value>,
}

impl Workflow {
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            nodes: Vec::new(),
            edges: Vec::new(),
            status: WorkflowStatus::Pending,
            created_at: now,
            updated_at: now,
            metadata: None,
        }
    }

    pub fn add_node(&mut self, node: WorkflowNode) {
        self.nodes.push(node);
        self.updated_at = chrono::Utc::now();
    }

    pub fn add_edge(&mut self, edge: WorkflowEdge) {
        self.edges.push(edge);
        self.updated_at = chrono::Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub config: serde_json::Value,
    pub inputs: HashMap<String, String>,
    pub outputs: HashMap<String, String>,
}

impl WorkflowNode {
    pub fn new(name: String, node_type: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            node_type,
            config: serde_json::Value::Null,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub source_output: Option<String>,
    pub target_input: Option<String>,
}

impl WorkflowEdge {
    pub fn new(source_node_id: String, target_node_id: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_node_id,
            target_node_id,
            source_output: None,
            target_input: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionContext {
    pub workflow_id: String,
    pub execution_id: String,
    pub node_contexts: HashMap<String, NodeExecutionContext>,
    pub global_state: serde_json::Value,
}

impl WorkflowExecutionContext {
    pub fn new(workflow_id: String) -> Self {
        Self {
            workflow_id,
            execution_id: Uuid::new_v4().to_string(),
            node_contexts: HashMap::new(),
            global_state: serde_json::Value::Null,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionContext {
    pub node_id: String,
    pub status: WorkflowStatus,
    pub inputs: HashMap<String, serde_json::Value>,
    pub outputs: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl NodeExecutionContext {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            status: WorkflowStatus::Pending,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            error: None,
            start_time: None,
            end_time: None,
        }
    }
}

#[async_trait]
pub trait WorkflowExecutor: Send + Sync {
    async fn execute(&self, workflow: &Workflow) -> crate::Result<WorkflowExecutionContext>;
    async fn pause(&self, execution_id: &str) -> crate::Result<()>;
    async fn resume(&self, execution_id: &str) -> crate::Result<()>;
    async fn cancel(&self, execution_id: &str) -> crate::Result<()>;
    async fn get_status(&self, execution_id: &str) -> crate::Result<WorkflowStatus>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStrategy {
    Serial,
    Parallel,
}

pub use dag::{DAGExecutor, DAGTopologicalSorter, DAGValidator};
pub use dsl::{
    ConditionDefinition, DSLFormat, DependencyDefinition, ExceptionHandlingConfig,
    ExceptionStrategy, LoopDefinition, RetryConfig, TaskDefinition, ValidationResult, WorkflowDSL,
    WorkflowDSLError, WorkflowDSLValidator,
};
pub use events::{WorkflowEvent, WorkflowEventBus, WorkflowEventType};
pub use execution::{
    DefaultTaskExecutor, ParallelExecutor, SerialExecutor, TaskExecutor, WorkflowExecutionEngine,
    WorkflowExecutionEngineBuilder,
};
pub use retry::{RetryPolicy, RetryStrategy};
pub use state::{WorkflowState, WorkflowStateManager};
pub use timeout::{TimeoutConfig, TimeoutManager};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_workflow_creation() {
        let workflow = Workflow::new("Test Workflow".to_string(), Some("Description".to_string()));
        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.description, Some("Description".to_string()));
        assert_eq!(workflow.nodes.len(), 0);
        assert_eq!(workflow.edges.len(), 0);
        assert_eq!(workflow.status, WorkflowStatus::Pending);
    }

    #[tokio::test]
    async fn test_workflow_node_creation() {
        let node = WorkflowNode::new("Test Node".to_string(), "task".to_string());
        assert_eq!(node.name, "Test Node");
        assert_eq!(node.node_type, "task");
    }

    #[tokio::test]
    async fn test_workflow_edge_creation() {
        let edge = WorkflowEdge::new("source".to_string(), "target".to_string());
        assert_eq!(edge.source_node_id, "source");
        assert_eq!(edge.target_node_id, "target");
    }

    #[tokio::test]
    async fn test_dag_validation_no_cycle() {
        let mut workflow = Workflow::new("Test Workflow".to_string(), None);
        let node1 = WorkflowNode::new("Node 1".to_string(), "task".to_string());
        let node2 = WorkflowNode::new("Node 2".to_string(), "task".to_string());
        let edge = WorkflowEdge::new(node1.id.clone(), node2.id.clone());

        workflow.add_node(node1);
        workflow.add_node(node2);
        workflow.add_edge(edge);

        let is_valid = DAGValidator::validate(&workflow).unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_dag_validation_with_cycle() {
        let mut workflow = Workflow::new("Test Workflow".to_string(), None);
        let node1 = WorkflowNode::new("Node 1".to_string(), "task".to_string());
        let node2 = WorkflowNode::new("Node 2".to_string(), "task".to_string());
        let edge1 = WorkflowEdge::new(node1.id.clone(), node2.id.clone());
        let edge2 = WorkflowEdge::new(node2.id.clone(), node1.id.clone());

        workflow.add_node(node1);
        workflow.add_node(node2);
        workflow.add_edge(edge1);
        workflow.add_edge(edge2);

        let is_valid = DAGValidator::validate(&workflow).unwrap();
        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_workflow_execution_engine_serial() {
        let state_manager = Arc::new(WorkflowStateManager::new());
        let event_bus = Arc::new(WorkflowEventBus::new());

        let engine = WorkflowExecutionEngine::builder()
            .with_strategy(ExecutionStrategy::Serial)
            .with_state_manager(state_manager.clone())
            .with_event_bus(event_bus.clone())
            .build();

        let mut workflow = Workflow::new("Test Serial Workflow".to_string(), None);
        let node1 = WorkflowNode::new("Node 1".to_string(), "task".to_string());
        let node2 = WorkflowNode::new("Node 2".to_string(), "task".to_string());
        let edge = WorkflowEdge::new(node1.id.clone(), node2.id.clone());

        workflow.add_node(node1);
        workflow.add_node(node2);
        workflow.add_edge(edge);

        let result = engine.execute(&workflow).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.node_contexts.len(), 2);

        let state = state_manager.get_workflow(&workflow.id).unwrap();
        assert_eq!(state.current_status, WorkflowStatus::Completed);
    }

    #[tokio::test]
    async fn test_workflow_execution_engine_parallel() {
        let state_manager = Arc::new(WorkflowStateManager::new());
        let event_bus = Arc::new(WorkflowEventBus::new());

        let engine = WorkflowExecutionEngine::builder()
            .with_strategy(ExecutionStrategy::Parallel)
            .with_state_manager(state_manager.clone())
            .with_event_bus(event_bus.clone())
            .build();

        let mut workflow = Workflow::new("Test Parallel Workflow".to_string(), None);
        let node1 = WorkflowNode::new("Node 1".to_string(), "task".to_string());
        let node2 = WorkflowNode::new("Node 2".to_string(), "task".to_string());

        workflow.add_node(node1);
        workflow.add_node(node2);

        let result = engine.execute(&workflow).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.node_contexts.len(), 2);

        let state = state_manager.get_workflow(&workflow.id).unwrap();
        assert_eq!(state.current_status, WorkflowStatus::Completed);
    }

    #[tokio::test]
    async fn test_workflow_with_retry_policy() {
        let state_manager = Arc::new(WorkflowStateManager::new());
        let event_bus = Arc::new(WorkflowEventBus::new());
        let retry_policy = RetryPolicy::new_fixed(3, Duration::from_millis(10));

        let engine = WorkflowExecutionEngine::builder()
            .with_strategy(ExecutionStrategy::Serial)
            .with_state_manager(state_manager.clone())
            .with_event_bus(event_bus.clone())
            .with_retry_policy(retry_policy)
            .build();

        let mut workflow = Workflow::new("Test Retry Workflow".to_string(), None);
        let node = WorkflowNode::new("Test Node".to_string(), "task".to_string());

        workflow.add_node(node);

        let result = engine.execute(&workflow).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_workflow_with_timeout_config() {
        let state_manager = Arc::new(WorkflowStateManager::new());
        let event_bus = Arc::new(WorkflowEventBus::new());
        let timeout_config = TimeoutConfig::new(Duration::from_secs(5));

        let engine = WorkflowExecutionEngine::builder()
            .with_strategy(ExecutionStrategy::Serial)
            .with_state_manager(state_manager.clone())
            .with_event_bus(event_bus.clone())
            .with_timeout_config(timeout_config)
            .build();

        let mut workflow = Workflow::new("Test Timeout Workflow".to_string(), None);
        let node = WorkflowNode::new("Test Node".to_string(), "task".to_string());

        workflow.add_node(node);

        let result = engine.execute(&workflow).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_state_manager() {
        let manager = WorkflowStateManager::new();
        let workflow = Workflow::new("Test Workflow".to_string(), None);
        let workflow_id = workflow.id.clone();

        let created_id = manager.create_workflow(workflow);
        assert_eq!(created_id, workflow_id);

        let state = manager.get_workflow(&workflow_id).unwrap();
        assert_eq!(state.workflow.id, workflow_id);
        assert_eq!(state.current_status, WorkflowStatus::Pending);

        let updated = manager.update_workflow_status(&workflow_id, WorkflowStatus::Running, None);
        assert!(updated);

        let state = manager.get_workflow(&workflow_id).unwrap();
        assert_eq!(state.current_status, WorkflowStatus::Running);

        let deleted = manager.delete_workflow(&workflow_id);
        assert!(deleted);

        let state = manager.get_workflow(&workflow_id);
        assert!(state.is_none());
    }

    #[tokio::test]
    async fn test_event_bus() {
        let bus = WorkflowEventBus::new();
        let workflow_id = Uuid::new_v4().to_string();

        let received_event = Arc::new(std::sync::Mutex::new(None));
        let received_event_clone = received_event.clone();

        bus.subscribe(WorkflowEventType::WorkflowStarted, move |event| {
            let mut received = received_event_clone.lock().unwrap();
            *received = Some(event.clone());
        });

        let event = WorkflowEvent::new(
            WorkflowEventType::WorkflowStarted,
            workflow_id.clone(),
            None,
            Some(WorkflowStatus::Running),
            Some("Test event".to_string()),
        );

        bus.publish(event.clone());

        tokio::time::sleep(Duration::from_millis(10)).await;

        let received = received_event.lock().unwrap();
        assert!(received.is_some());
        assert_eq!(
            received.as_ref().unwrap().event_type,
            WorkflowEventType::WorkflowStarted
        );
        assert_eq!(received.as_ref().unwrap().workflow_id, workflow_id);
    }

    #[tokio::test]
    async fn test_topological_sort() {
        let mut workflow = Workflow::new("Test Sort Workflow".to_string(), None);
        let node1 = WorkflowNode::new("Node 1".to_string(), "task".to_string());
        let node2 = WorkflowNode::new("Node 2".to_string(), "task".to_string());
        let node3 = WorkflowNode::new("Node 3".to_string(), "task".to_string());
        let edge1 = WorkflowEdge::new(node1.id.clone(), node2.id.clone());
        let edge2 = WorkflowEdge::new(node2.id.clone(), node3.id.clone());

        workflow.add_node(node1.clone());
        workflow.add_node(node2.clone());
        workflow.add_node(node3.clone());
        workflow.add_edge(edge1);
        workflow.add_edge(edge2);

        let sorted = DAGTopologicalSorter::sort(&workflow).unwrap();
        assert_eq!(sorted.len(), 3);

        let node1_pos = sorted.iter().position(|id| id == &node1.id).unwrap();
        let node2_pos = sorted.iter().position(|id| id == &node2.id).unwrap();
        let node3_pos = sorted.iter().position(|id| id == &node3.id).unwrap();

        assert!(node1_pos < node2_pos);
        assert!(node2_pos < node3_pos);
    }
}
