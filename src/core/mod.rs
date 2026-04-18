pub mod cache;
pub mod coordinator;
pub mod edge_cdn;
pub mod intent;
pub mod llm;
pub mod performance;
pub mod plan_execute;
pub mod planner;
pub mod plugin;
pub mod progressive_loading;
pub mod realtime_scheduler;
pub mod reflect;
pub mod scheduler;
pub mod smart_preload;
pub mod types;

pub use types::{ExecutionContext, Task, TaskExecutor, TaskStatus};
pub use plan_execute::{
    PlanAndExecuteEngine, ReActEngine, PlanExecuteState, PlanExecuteStatus,
    PlanExecuteResult, ReActStep, ReActStepType,
};

use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::core::llm::{LlmManager, MockLlmAdapter};
use crate::utils::Result;
use intent::{Constraint, Intent, IntentParser, ValidationResult};
use planner::{ExecutionPlan, TaskPlanner};
use progressive_loading::{LoadingStrategy, LoadingSummary, ProgressiveLoader, TokenUsage};
use reflect::{ExecutionReport, Reflector};
use scheduler::TaskScheduler;

pub struct CommanderCore {
    intent_parser: Arc<IntentParser>,
    task_planner: Arc<Mutex<TaskPlanner>>,
    task_scheduler: Arc<Mutex<TaskScheduler>>,
    reflector: Arc<Reflector>,
    executors: Vec<Box<dyn TaskExecutor>>,
    execution_contexts: Arc<Mutex<Vec<ExecutionContext>>>,
    progressive_loader: Arc<ProgressiveLoader>,
    llm_manager: Arc<LlmManager>,
}

impl CommanderCore {
    pub fn new() -> Self {
        let llm_manager = Arc::new(LlmManager::new());
        let mock_adapter = Arc::new(MockLlmAdapter::new());
        llm_manager.register_adapter(mock_adapter);

        let intent_parser = Arc::new(IntentParser::new().with_llm_manager(llm_manager.clone()));

        Self {
            intent_parser,
            task_planner: Arc::new(Mutex::new(TaskPlanner::new())),
            task_scheduler: Arc::new(Mutex::new(TaskScheduler::new())),
            reflector: Arc::new(Reflector::new()),
            executors: Vec::new(),
            execution_contexts: Arc::new(Mutex::new(Vec::new())),
            progressive_loader: Arc::new(ProgressiveLoader::default()),
            llm_manager,
        }
    }

    pub fn with_llm_manager(mut self, llm_manager: LlmManager) -> Self {
        let llm_manager = Arc::new(llm_manager);
        self.llm_manager = llm_manager.clone();
        self.intent_parser = Arc::new(IntentParser::new().with_llm_manager(llm_manager.clone()));
        self
    }

    pub fn llm_manager(&self) -> Arc<LlmManager> {
        self.llm_manager.clone()
    }

    pub fn with_progressive_loader(mut self, loader: ProgressiveLoader) -> Self {
        self.progressive_loader = Arc::new(loader);
        self
    }

    pub async fn create_progressive_loading_context(
        &self,
        task: &Task,
        strategy: LoadingStrategy,
        max_depth: u32,
    ) -> Result<progressive_loading::LoadingContext> {
        self.progressive_loader
            .create_context(task, strategy, max_depth)
            .await
    }

    pub async fn get_token_usage(&self, task_id: &str) -> Option<TokenUsage> {
        self.progressive_loader.get_token_usage(task_id).await
    }

    pub async fn get_loading_summary(&self, task_id: &str) -> Option<LoadingSummary> {
        self.progressive_loader.get_loading_summary(task_id).await
    }

    pub async fn is_within_token_budget(&self, task_id: &str) -> bool {
        self.progressive_loader.is_within_budget(task_id).await
    }

    pub fn with_intent_parser(mut self, parser: IntentParser) -> Self {
        self.intent_parser = Arc::new(parser);
        self
    }

    pub fn register_executor(&mut self, executor: Box<dyn TaskExecutor>) {
        self.executors.push(executor);
    }

    pub async fn process_intent(&self, raw_input: &str) -> Result<(Intent, ValidationResult)> {
        info!("Processing intent: {}", raw_input);

        let intent = self.intent_parser.parse(raw_input).await?;
        let validation = self.intent_parser.validate_intent(&intent)?;

        Ok((intent, validation))
    }

    pub async fn create_plan_from_intent(&self, intent: Intent) -> Result<ExecutionPlan> {
        info!("Creating execution plan from intent: {}", intent.intent_id);

        let root_task = self.intent_parser.to_task(intent)?;

        let planner = self.task_planner.lock().await;
        let planner_with_llm = planner.clone().with_llm_manager(self.llm_manager.clone());
        let sub_tasks = planner_with_llm.decompose_task(&root_task).await?;

        let mut planner = self.task_planner.lock().await;
        let plan = planner.plan(root_task, sub_tasks)?;

        planner.validate_plan(&plan)?;

        Ok(plan)
    }

    pub async fn execute_plan(&self, plan: ExecutionPlan) -> Result<ExecutionContext> {
        info!("Executing plan: {}", plan.plan_id);

        let mut context = ExecutionContext::new().with_execution_plan(plan.clone());
        let mut scheduler = self.task_scheduler.lock().await;

        let mut plan = plan;

        for node in plan.nodes.values() {
            scheduler.schedule(node.task.clone());
        }

        self.execution_contexts.lock().await.push(context.clone());

        while !plan.is_complete() && !plan.has_failed_tasks() {
            let ready_node_ids: Vec<String> = plan
                .get_ready_nodes()
                .iter()
                .map(|node| node.node_id.clone())
                .collect();

            for node_id in ready_node_ids {
                if let Some(node) = plan.get_node(&node_id).cloned() {
                    if let Some(node_mut) = plan.get_node_mut(&node_id) {
                        node_mut.mark_running();
                    }
                    context.update_current_task(node_id.clone(), context.current_step + 1);

                    let mut task = node.task.clone();
                    task.mark_running();

                    let execution_result = self.execute_task(task).await;

                    match execution_result {
                        Ok(mut completed_task) => {
                            completed_task.mark_completed();
                            if let Some(node_mut) = plan.get_node_mut(&node_id) {
                                node_mut.mark_completed(1000);
                            }
                            info!("Task completed: {}", node_id);
                        }
                        Err(e) => {
                            warn!("Task failed: {} - Error: {}", node_id, e);
                            if let Some(node_mut) = plan.get_node_mut(&node_id) {
                                node_mut.mark_failed();
                            }

                            let mut planner = self.task_planner.lock().await;
                            plan = planner.replan(plan, &node_id)?;
                        }
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        if plan.is_complete() {
            info!("Plan completed successfully: {}", plan.plan_id);
        } else if plan.has_failed_tasks() {
            warn!("Plan has failed tasks: {}", plan.plan_id);
        }

        Ok(context)
    }

    async fn execute_task(&self, mut task: Task) -> Result<Task> {
        for executor in &self.executors {
            if executor.can_execute(&task) {
                return executor.execute(task).await;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        task.mark_completed();
        Ok(task)
    }

    pub async fn submit_task(&self, task: Task) -> Result<Task> {
        info!("Submitting task: {}", task.id);

        let mut scheduler = self.task_scheduler.lock().await;
        scheduler.schedule(task.clone());

        Ok(task)
    }

    pub async fn get_next_task(&self) -> Option<Task> {
        let mut scheduler = self.task_scheduler.lock().await;
        scheduler.pop_task()
    }

    pub async fn reflect_on_execution(&self, task: &Task) -> Result<ExecutionReport> {
        info!("Reflecting on task execution: {}", task.id);
        self.reflector.analyze(task).await
    }

    pub fn add_common_constraint(&self, constraint: Constraint) {
        let mut parser = IntentParser::new();
        parser.add_common_constraint(constraint);
    }

    pub fn get_execution_contexts(&self) -> Arc<Mutex<Vec<ExecutionContext>>> {
        self.execution_contexts.clone()
    }
}

impl Default for CommanderCore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commander_core_new() {
        let core = CommanderCore::new();
        assert!(core.executors.is_empty());
    }

    #[test]
    fn test_commander_core_default() {
        let core = CommanderCore::default();
        assert!(core.executors.is_empty());
    }

    #[test]
    fn test_commander_core_with_llm_manager() {
        let llm_manager = crate::core::llm::LlmManager::new();
        let core = CommanderCore::new().with_llm_manager(llm_manager);
        let llm = core.llm_manager();
        assert!(llm.adapters().is_empty());
    }
}
