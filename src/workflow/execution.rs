use crate::Result;
use crate::workflow::{
    DAGTopologicalSorter, DAGValidator, ExecutionStrategy, NodeExecutionContext, RetryPolicy,
    TimeoutConfig, TimeoutManager, Workflow, WorkflowEvent, WorkflowEventBus, WorkflowEventType,
    WorkflowExecutionContext, WorkflowNode, WorkflowStateManager, WorkflowStatus,
};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

#[async_trait]
pub trait TaskExecutor: Send + Sync {
    async fn execute_task(
        &self,
        node: &WorkflowNode,
        context: &mut WorkflowExecutionContext,
    ) -> Result<NodeExecutionContext>;
}

pub struct DefaultTaskExecutor;

#[async_trait]
impl TaskExecutor for DefaultTaskExecutor {
    async fn execute_task(
        &self,
        node: &WorkflowNode,
        _context: &mut WorkflowExecutionContext,
    ) -> Result<NodeExecutionContext> {
        let mut node_context = NodeExecutionContext::new(node.id.clone());
        node_context.status = WorkflowStatus::Running;
        node_context.start_time = Some(chrono::Utc::now());

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        node_context.status = WorkflowStatus::Completed;
        node_context.end_time = Some(chrono::Utc::now());

        Ok(node_context)
    }
}

pub struct SerialExecutor;

#[async_trait]
impl TaskExecutor for SerialExecutor {
    async fn execute_task(
        &self,
        node: &WorkflowNode,
        context: &mut WorkflowExecutionContext,
    ) -> Result<NodeExecutionContext> {
        DefaultTaskExecutor.execute_task(node, context).await
    }
}

pub struct ParallelExecutor {
    max_concurrent: usize,
}

impl ParallelExecutor {
    pub fn new(max_concurrent: usize) -> Self {
        Self { max_concurrent }
    }
}

#[async_trait]
impl TaskExecutor for ParallelExecutor {
    async fn execute_task(
        &self,
        node: &WorkflowNode,
        context: &mut WorkflowExecutionContext,
    ) -> Result<NodeExecutionContext> {
        DefaultTaskExecutor.execute_task(node, context).await
    }
}

pub struct WorkflowExecutionEngine {
    strategy: ExecutionStrategy,
    task_executor: Arc<dyn TaskExecutor>,
    state_manager: Arc<WorkflowStateManager>,
    event_bus: Arc<WorkflowEventBus>,
    retry_policy: Option<RetryPolicy>,
    timeout_config: Option<TimeoutConfig>,
    timeout_manager: TimeoutManager,
}

impl WorkflowExecutionEngine {
    pub fn new(
        strategy: ExecutionStrategy,
        task_executor: Arc<dyn TaskExecutor>,
        state_manager: Arc<WorkflowStateManager>,
        event_bus: Arc<WorkflowEventBus>,
    ) -> Self {
        Self {
            strategy,
            task_executor,
            state_manager,
            event_bus,
            retry_policy: None,
            timeout_config: None,
            timeout_manager: TimeoutManager::new(),
        }
    }

    pub fn with_retry_policy(mut self, retry_policy: RetryPolicy) -> Self {
        self.retry_policy = Some(retry_policy);
        self
    }

    pub fn with_timeout_config(mut self, timeout_config: TimeoutConfig) -> Self {
        self.timeout_config = Some(timeout_config);
        self
    }

    pub fn builder() -> WorkflowExecutionEngineBuilder {
        WorkflowExecutionEngineBuilder::new()
    }

    fn build_dependency_graph(
        workflow: &Workflow,
    ) -> (HashMap<String, Vec<String>>, HashMap<String, Vec<String>>) {
        let mut dependencies: HashMap<String, Vec<String>> = HashMap::new();
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

        for node in &workflow.nodes {
            dependencies.insert(node.id.clone(), Vec::new());
            dependents.insert(node.id.clone(), Vec::new());
        }

        for edge in &workflow.edges {
            if let Some(deps) = dependencies.get_mut(&edge.target_node_id) {
                deps.push(edge.source_node_id.clone());
            }
            if let Some(dep_list) = dependents.get_mut(&edge.source_node_id) {
                dep_list.push(edge.target_node_id.clone());
            }
        }

        (dependencies, dependents)
    }

    async fn execute_node_with_retry_and_timeout(
        &self,
        workflow: &Workflow,
        node_id: String,
        context: &mut WorkflowExecutionContext,
    ) -> Result<()> {
        let node = workflow
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .ok_or_else(|| crate::AetherisError::Internal(format!("Node {} not found", node_id)))?;

        self.publish_node_event(
            workflow.id.clone(),
            node_id.clone(),
            WorkflowEventType::NodeStarted,
            Some(WorkflowStatus::Running),
            Some("Node execution started".to_string()),
        );

        let mut node_context = NodeExecutionContext::new(node_id.clone());
        node_context.status = WorkflowStatus::Running;
        node_context.start_time = Some(chrono::Utc::now());
        context.node_contexts.insert(node_id.clone(), node_context);

        let task_executor = self.task_executor.clone();
        let node_clone = node.clone();

        let execute_with_timeout = || async {
            if let Some(ref timeout_config) = self.timeout_config {
                let task_executor = task_executor.clone();
                let node_clone = node_clone.clone();
                let mut ctx = WorkflowExecutionContext::new(workflow.id.clone());

                self.timeout_manager
                    .with_timeout(
                        timeout_config.clone(),
                        async move { task_executor.execute_task(&node_clone, &mut ctx).await },
                        None,
                    )
                    .await
            } else {
                let mut ctx = WorkflowExecutionContext::new(workflow.id.clone());
                task_executor.execute_task(&node_clone, &mut ctx).await
            }
        };

        let result_exec = if let Some(ref retry_policy) = self.retry_policy {
            retry_policy.execute_with_retry(execute_with_timeout).await
        } else {
            execute_with_timeout().await
        };

        match result_exec {
            Ok(mut node_ctx) => {
                node_ctx.end_time = Some(chrono::Utc::now());
                node_ctx.status = WorkflowStatus::Completed;
                context.node_contexts.insert(node_id.clone(), node_ctx);

                self.publish_node_event(
                    workflow.id.clone(),
                    node_id.clone(),
                    WorkflowEventType::NodeCompleted,
                    Some(WorkflowStatus::Completed),
                    Some("Node execution completed".to_string()),
                );
                Ok(())
            }
            Err(e) => {
                if let Some(mut node_ctx) = context.node_contexts.get_mut(&node_id) {
                    node_ctx.status = WorkflowStatus::Failed;
                    node_ctx.end_time = Some(chrono::Utc::now());
                    node_ctx.error = Some(e.to_string());
                }
                self.publish_node_event(
                    workflow.id.clone(),
                    node_id.clone(),
                    WorkflowEventType::NodeFailed,
                    Some(WorkflowStatus::Failed),
                    Some(format!("Node execution failed: {}", e)),
                );
                Err(e)
            }
        }
    }

    async fn execute_serial(
        &self,
        workflow: &Workflow,
        context: &mut WorkflowExecutionContext,
    ) -> Result<()> {
        let sorted_nodes = DAGTopologicalSorter::sort(workflow)?;

        for node_id in sorted_nodes {
            self.execute_node_with_retry_and_timeout(workflow, node_id, context)
                .await?;
            self.state_manager
                .set_execution_context(&workflow.id, context.clone());
        }

        Ok(())
    }

    async fn execute_parallel(
        &self,
        workflow: &Workflow,
        context: &mut WorkflowExecutionContext,
    ) -> Result<()> {
        let (dependencies, _) = Self::build_dependency_graph(workflow);
        let mut completed_nodes: HashSet<String> = HashSet::new();
        let mut in_progress_nodes: HashSet<String> = HashSet::new();

        while completed_nodes.len() < workflow.nodes.len() {
            let mut ready_nodes = Vec::new();

            for node in &workflow.nodes {
                if !completed_nodes.contains(&node.id) && !in_progress_nodes.contains(&node.id) {
                    let deps = dependencies.get(&node.id).unwrap();
                    let all_deps_completed = deps.iter().all(|dep| completed_nodes.contains(dep));

                    if all_deps_completed {
                        ready_nodes.push(node.id.clone());
                    }
                }
            }

            if ready_nodes.is_empty() {
                break;
            }

            let mut handles = Vec::new();
            for node_id in ready_nodes {
                in_progress_nodes.insert(node_id.clone());
                let node_id_clone = node_id.clone();
                let workflow_clone = workflow.clone();
                let mut context_clone = context.clone();
                let self_clone = self.clone();

                handles.push(tokio::spawn(async move {
                    let result = self_clone
                        .execute_node_with_retry_and_timeout(
                            &workflow_clone,
                            node_id_clone,
                            &mut context_clone,
                        )
                        .await;
                    (node_id, result, context_clone)
                }));
            }

            for handle in handles {
                let (node_id, result, updated_context) = handle.await?;
                in_progress_nodes.remove(&node_id);
                match result {
                    Ok(_) => {
                        completed_nodes.insert(node_id);
                        context.node_contexts.extend(updated_context.node_contexts);
                        self.state_manager
                            .set_execution_context(&workflow.id, context.clone());
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(())
    }

    fn publish_workflow_event(
        &self,
        workflow_id: String,
        event_type: WorkflowEventType,
        status: Option<WorkflowStatus>,
        message: Option<String>,
    ) {
        let event = WorkflowEvent::new(event_type, workflow_id, None, status, message);
        self.event_bus.publish(event);
    }

    fn publish_node_event(
        &self,
        workflow_id: String,
        node_id: String,
        event_type: WorkflowEventType,
        status: Option<WorkflowStatus>,
        message: Option<String>,
    ) {
        let event = WorkflowEvent::new(event_type, workflow_id, Some(node_id), status, message);
        self.event_bus.publish(event);
    }

    pub async fn resume_from_breakpoint(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowExecutionContext> {
        let state = self
            .state_manager
            .get_workflow(workflow_id)
            .ok_or_else(|| crate::AetherisError::Internal("Workflow not found".to_string()))?;

        let mut context = state.execution_context.ok_or_else(|| {
            crate::AetherisError::Internal("No execution context found".to_string())
        })?;

        self.publish_workflow_event(
            workflow_id.to_string(),
            WorkflowEventType::WorkflowResumed,
            Some(WorkflowStatus::Running),
            Some("Workflow resumed from breakpoint".to_string()),
        );
        self.state_manager
            .update_workflow_status(workflow_id, WorkflowStatus::Running, None);

        match self.strategy {
            ExecutionStrategy::Serial => self.execute_serial(&state.workflow, &mut context).await?,
            ExecutionStrategy::Parallel => {
                self.execute_parallel(&state.workflow, &mut context).await?
            }
        }

        self.publish_workflow_event(
            workflow_id.to_string(),
            WorkflowEventType::WorkflowCompleted,
            Some(WorkflowStatus::Completed),
            Some("Workflow completed successfully".to_string()),
        );
        self.state_manager
            .update_workflow_status(workflow_id, WorkflowStatus::Completed, None);
        self.state_manager
            .set_execution_context(workflow_id, context.clone());

        Ok(context)
    }
}

impl Clone for WorkflowExecutionEngine {
    fn clone(&self) -> Self {
        Self {
            strategy: self.strategy.clone(),
            task_executor: self.task_executor.clone(),
            state_manager: self.state_manager.clone(),
            event_bus: self.event_bus.clone(),
            retry_policy: self.retry_policy.clone(),
            timeout_config: self.timeout_config.clone(),
            timeout_manager: TimeoutManager::new(),
        }
    }
}

#[async_trait]
impl crate::workflow::WorkflowExecutor for WorkflowExecutionEngine {
    async fn execute(&self, workflow: &Workflow) -> Result<WorkflowExecutionContext> {
        if !DAGValidator::validate(workflow)? {
            return Err(crate::AetherisError::Validation(
                "Workflow contains cycles".to_string(),
            ));
        }

        self.state_manager.create_workflow(workflow.clone());

        let mut context = WorkflowExecutionContext::new(workflow.id.clone());

        self.publish_workflow_event(
            workflow.id.clone(),
            WorkflowEventType::WorkflowStarted,
            Some(WorkflowStatus::Running),
            Some("Workflow execution started".to_string()),
        );
        self.state_manager
            .update_workflow_status(&workflow.id, WorkflowStatus::Running, None);

        let result = match self.strategy {
            ExecutionStrategy::Serial => self.execute_serial(workflow, &mut context).await,
            ExecutionStrategy::Parallel => self.execute_parallel(workflow, &mut context).await,
        };

        match result {
            Ok(_) => {
                self.publish_workflow_event(
                    workflow.id.clone(),
                    WorkflowEventType::WorkflowCompleted,
                    Some(WorkflowStatus::Completed),
                    Some("Workflow completed successfully".to_string()),
                );
                self.state_manager.update_workflow_status(
                    &workflow.id,
                    WorkflowStatus::Completed,
                    None,
                );
            }
            Err(ref e) => {
                self.publish_workflow_event(
                    workflow.id.clone(),
                    WorkflowEventType::WorkflowFailed,
                    Some(WorkflowStatus::Failed),
                    Some(format!("Workflow failed: {}", e)),
                );
                self.state_manager.update_workflow_status(
                    &workflow.id,
                    WorkflowStatus::Failed,
                    Some(e.to_string()),
                );
            }
        }

        self.state_manager
            .set_execution_context(&workflow.id, context.clone());

        result.map(|_| context)
    }

    async fn pause(&self, execution_id: &str) -> Result<()> {
        self.publish_workflow_event(
            execution_id.to_string(),
            WorkflowEventType::WorkflowPaused,
            Some(WorkflowStatus::Paused),
            Some("Workflow paused".to_string()),
        );
        self.state_manager
            .update_workflow_status(execution_id, WorkflowStatus::Paused, None);
        Ok(())
    }

    async fn resume(&self, execution_id: &str) -> Result<()> {
        self.resume_from_breakpoint(execution_id).await?;
        Ok(())
    }

    async fn cancel(&self, execution_id: &str) -> Result<()> {
        self.publish_workflow_event(
            execution_id.to_string(),
            WorkflowEventType::WorkflowCancelled,
            Some(WorkflowStatus::Cancelled),
            Some("Workflow cancelled".to_string()),
        );
        self.state_manager
            .update_workflow_status(execution_id, WorkflowStatus::Cancelled, None);
        Ok(())
    }

    async fn get_status(&self, execution_id: &str) -> Result<WorkflowStatus> {
        self.state_manager
            .get_status(execution_id)
            .ok_or_else(|| crate::AetherisError::Internal("Workflow not found".to_string()))
    }
}

pub struct WorkflowExecutionEngineBuilder {
    strategy: Option<ExecutionStrategy>,
    task_executor: Option<Arc<dyn TaskExecutor>>,
    state_manager: Option<Arc<WorkflowStateManager>>,
    event_bus: Option<Arc<WorkflowEventBus>>,
    retry_policy: Option<RetryPolicy>,
    timeout_config: Option<TimeoutConfig>,
}

impl WorkflowExecutionEngineBuilder {
    pub fn new() -> Self {
        Self {
            strategy: None,
            task_executor: None,
            state_manager: None,
            event_bus: None,
            retry_policy: None,
            timeout_config: None,
        }
    }

    pub fn with_strategy(mut self, strategy: ExecutionStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    pub fn with_task_executor(mut self, executor: Arc<dyn TaskExecutor>) -> Self {
        self.task_executor = Some(executor);
        self
    }

    pub fn with_state_manager(mut self, manager: Arc<WorkflowStateManager>) -> Self {
        self.state_manager = Some(manager);
        self
    }

    pub fn with_event_bus(mut self, bus: Arc<WorkflowEventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    pub fn with_timeout_config(mut self, config: TimeoutConfig) -> Self {
        self.timeout_config = Some(config);
        self
    }

    pub fn build(self) -> WorkflowExecutionEngine {
        let strategy = self.strategy.unwrap_or(ExecutionStrategy::Serial);
        let task_executor = self
            .task_executor
            .unwrap_or_else(|| Arc::new(DefaultTaskExecutor));
        let state_manager = self
            .state_manager
            .unwrap_or_else(|| Arc::new(WorkflowStateManager::new()));
        let event_bus = self
            .event_bus
            .unwrap_or_else(|| Arc::new(WorkflowEventBus::new()));

        let mut engine =
            WorkflowExecutionEngine::new(strategy, task_executor, state_manager, event_bus);

        if let Some(policy) = self.retry_policy {
            engine = engine.with_retry_policy(policy);
        }

        if let Some(config) = self.timeout_config {
            engine = engine.with_timeout_config(config);
        }

        engine
    }
}

impl Default for WorkflowExecutionEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
