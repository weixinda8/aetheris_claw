use crate::core::Task;
use crate::core::llm::LlmManager;
use crate::utils::{AetherisError, Result};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaskDependencyType {
    Hard,
    Soft,
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependency {
    pub source_task_id: String,
    pub target_task_id: String,
    pub dependency_type: TaskDependencyType,
    pub description: String,
}

impl TaskDependency {
    pub fn new(
        source_task_id: String,
        target_task_id: String,
        dependency_type: TaskDependencyType,
    ) -> Self {
        Self {
            source_task_id,
            target_task_id,
            dependency_type,
            description: String::new(),
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskNodeStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    pub node_id: String,
    pub task: Task,
    pub dependencies: Vec<TaskDependency>,
    pub dependents: Vec<String>,
    pub status: TaskNodeStatus,
    pub estimated_duration_ms: u64,
    pub actual_duration_ms: Option<u64>,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl TaskNode {
    pub fn new(task: Task) -> Self {
        Self {
            node_id: task.id.clone(),
            task,
            dependencies: Vec::new(),
            dependents: Vec::new(),
            status: TaskNodeStatus::Pending,
            estimated_duration_ms: 1000,
            actual_duration_ms: None,
            retry_count: 0,
            max_retries: 3,
        }
    }

    pub fn with_estimated_duration(mut self, duration: u64) -> Self {
        self.estimated_duration_ms = duration;
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn add_dependency(&mut self, dependency: TaskDependency) {
        self.dependencies.push(dependency);
    }

    pub fn add_dependent(&mut self, dependent_id: String) {
        self.dependents.push(dependent_id);
    }

    pub fn can_execute(&self, completed_tasks: &HashSet<String>) -> bool {
        if self.status != TaskNodeStatus::Pending && self.status != TaskNodeStatus::Ready {
            return false;
        }

        self.dependencies.iter().all(|dep| {
            if dep.dependency_type == TaskDependencyType::Hard {
                completed_tasks.contains(&dep.source_task_id)
            } else {
                true
            }
        })
    }

    pub fn mark_ready(&mut self) {
        if self.status == TaskNodeStatus::Pending {
            self.status = TaskNodeStatus::Ready;
        }
    }

    pub fn mark_running(&mut self) {
        self.status = TaskNodeStatus::Running;
    }

    pub fn mark_completed(&mut self, duration_ms: u64) {
        self.status = TaskNodeStatus::Completed;
        self.actual_duration_ms = Some(duration_ms);
    }

    pub fn mark_failed(&mut self) {
        self.status = TaskNodeStatus::Failed;
        self.retry_count += 1;
    }

    pub fn can_retry(&self) -> bool {
        self.status == TaskNodeStatus::Failed && self.retry_count < self.max_retries
    }

    pub fn reset_for_retry(&mut self) {
        if self.can_retry() {
            self.status = TaskNodeStatus::Pending;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub plan_id: String,
    pub root_task_id: String,
    pub nodes: HashMap<String, TaskNode>,
    pub dependencies: Vec<TaskDependency>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ExecutionPlan {
    pub fn new(root_task_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            plan_id: uuid::Uuid::new_v4().to_string(),
            root_task_id,
            nodes: HashMap::new(),
            dependencies: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_node(&mut self, node: TaskNode) {
        self.nodes.insert(node.node_id.clone(), node);
        self.updated_at = chrono::Utc::now();
    }

    pub fn add_dependency(&mut self, dependency: TaskDependency) {
        if let Some(target_node) = self.nodes.get_mut(&dependency.target_task_id) {
            target_node.add_dependency(dependency.clone());
        }
        if let Some(source_node) = self.nodes.get_mut(&dependency.source_task_id) {
            source_node.add_dependent(dependency.target_task_id.clone());
        }
        self.dependencies.push(dependency);
        self.updated_at = chrono::Utc::now();
    }

    pub fn get_node(&self, node_id: &str) -> Option<&TaskNode> {
        self.nodes.get(node_id)
    }

    pub fn get_node_mut(&mut self, node_id: &str) -> Option<&mut TaskNode> {
        self.nodes.get_mut(node_id)
    }

    pub fn get_ready_nodes(&self) -> Vec<&TaskNode> {
        let completed_tasks: HashSet<String> = self
            .nodes
            .iter()
            .filter(|(_, node)| node.status == TaskNodeStatus::Completed)
            .map(|(id, _)| id.clone())
            .collect();

        self.nodes
            .iter()
            .filter(|(_, node)| node.can_execute(&completed_tasks))
            .map(|(_, node)| node)
            .collect()
    }

    pub fn is_complete(&self) -> bool {
        self.nodes.values().all(|node| {
            node.status == TaskNodeStatus::Completed || node.status == TaskNodeStatus::Skipped
        })
    }

    pub fn has_failed_tasks(&self) -> bool {
        self.nodes
            .values()
            .any(|node| node.status == TaskNodeStatus::Failed)
    }
}

#[derive(Clone)]
pub struct TaskPlanner {
    graph: DiGraph<TaskNode, TaskDependency>,
    node_indices: HashMap<String, NodeIndex>,
    llm_manager: Option<Arc<LlmManager>>,
}

impl TaskPlanner {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
            llm_manager: None,
        }
    }

    pub fn with_llm_manager(mut self, llm_manager: Arc<LlmManager>) -> Self {
        self.llm_manager = Some(llm_manager);
        self
    }

    pub fn plan(&mut self, root_task: Task, sub_tasks: Vec<Task>) -> Result<ExecutionPlan> {
        info!("Creating execution plan for root task: {}", root_task.id);

        let mut plan = ExecutionPlan::new(root_task.id.clone());

        let root_node = TaskNode::new(root_task);
        plan.add_node(root_node);

        for sub_task in sub_tasks {
            let sub_node = TaskNode::new(sub_task);
            plan.add_node(sub_node);
        }

        self.build_dependency_graph(&mut plan)?;
        self.estimate_durations(&mut plan);

        debug!("Execution plan created with {} nodes", plan.nodes.len());
        Ok(plan)
    }

    fn build_dependency_graph(&mut self, plan: &mut ExecutionPlan) -> Result<()> {
        self.graph.clear();
        self.node_indices.clear();

        let node_ids: Vec<String> = plan.nodes.keys().cloned().collect();
        let root_id = plan.root_task_id.clone();

        for node_id in &node_ids {
            if let Some(task_node) = plan.nodes.get(node_id) {
                let node_index = self.graph.add_node(task_node.clone());
                self.node_indices.insert(node_id.clone(), node_index);
            }
        }

        if node_ids.len() <= 1 {
            return Ok(());
        }

        for node_id in &node_ids {
            if node_id != &root_id {
                let dependency =
                    TaskDependency::new(node_id.clone(), root_id.clone(), TaskDependencyType::Hard);
                plan.add_dependency(dependency.clone());

                if let (Some(source_idx), Some(target_idx)) = (
                    self.node_indices.get(&dependency.source_task_id),
                    self.node_indices.get(&dependency.target_task_id),
                ) {
                    self.graph.add_edge(*source_idx, *target_idx, dependency);
                }
            }
        }

        Ok(())
    }

    fn estimate_durations(&self, plan: &mut ExecutionPlan) {
        for node in plan.nodes.values_mut() {
            let base_duration = 1000;
            let priority_factor = (10 - node.task.priority) as u64 * 100;
            node.estimated_duration_ms = base_duration + priority_factor;
        }
    }

    pub fn replan(
        &mut self,
        mut plan: ExecutionPlan,
        failed_task_id: &str,
    ) -> Result<ExecutionPlan> {
        info!("Replanning after failure of task: {}", failed_task_id);

        let (can_retry, dependents) = {
            if let Some(failed_node) = plan.get_node(failed_task_id) {
                (failed_node.can_retry(), failed_node.dependents.clone())
            } else {
                (false, Vec::new())
            }
        };

        if can_retry {
            if let Some(node) = plan.get_node_mut(failed_task_id) {
                node.reset_for_retry();
            }
            info!("Task will be retried: {}", failed_task_id);
            plan.updated_at = chrono::Utc::now();
            return Ok(plan);
        }

        for dep in &dependents {
            if let Some(node) = plan.get_node_mut(dep) {
                let has_hard_dependency_on_failed = node.dependencies.iter().any(|d| {
                    d.source_task_id == failed_task_id
                        && d.dependency_type == TaskDependencyType::Hard
                });
                if has_hard_dependency_on_failed {
                    node.status = TaskNodeStatus::Skipped;
                    warn!("Dependent task skipped: {}", dep);
                }
            }
        }

        plan.updated_at = chrono::Utc::now();
        Ok(plan)
    }

    pub fn get_execution_order(&self, plan: &ExecutionPlan) -> Result<Vec<String>> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();

        fn visit(
            node_id: &str,
            plan: &ExecutionPlan,
            order: &mut Vec<String>,
            visited: &mut HashSet<String>,
        ) {
            if visited.contains(node_id) {
                return;
            }

            visited.insert(node_id.to_string());

            if let Some(node) = plan.get_node(node_id) {
                for dep in &node.dependencies {
                    visit(&dep.source_task_id, plan, order, visited);
                }
            }

            order.push(node_id.to_string());
        }

        visit(&plan.root_task_id, plan, &mut order, &mut visited);

        Ok(order)
    }

    pub async fn decompose_task(&self, task: &Task) -> Result<Vec<Task>> {
        info!("Decomposing task: {}", task.id);

        if let Some(llm_manager) = &self.llm_manager {
            self.decompose_task_with_llm(task, llm_manager).await
        } else {
            self.decompose_task_rule_based(task)
        }
    }

    pub fn decompose_task_sync(&self, task: &Task) -> Result<Vec<Task>> {
        info!("Decomposing task (sync): {}", task.id);
        self.decompose_task_rule_based(task)
    }

    fn decompose_task_rule_based(&self, task: &Task) -> Result<Vec<Task>> {
        let mut sub_tasks = Vec::new();

        let task_parts = task
            .description
            .split(['.', ';', '\n'])
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<_>>();

        for (i, part) in task_parts.iter().enumerate() {
            let mut sub_task = Task::new(part.trim().to_string(), task.priority);
            let mut metadata = serde_json::Map::new();
            metadata.insert(
                "parent_task_id".to_string(),
                serde_json::Value::String(task.id.clone()),
            );
            metadata.insert(
                "step".to_string(),
                serde_json::Value::Number(serde_json::Number::from(i + 1)),
            );
            sub_task.metadata = serde_json::Value::Object(metadata);
            sub_tasks.push(sub_task);
        }

        if sub_tasks.is_empty() {
            sub_tasks.push(task.clone());
        }

        debug!(
            "Task decomposed into {} subtasks (rule-based)",
            sub_tasks.len()
        );
        Ok(sub_tasks)
    }

    async fn decompose_task_with_llm(
        &self,
        task: &Task,
        llm_manager: &Arc<LlmManager>,
    ) -> Result<Vec<Task>> {
        info!("Decomposing task with LLM: {}", task.id);

        let system_prompt = r#"You are a task decomposition expert for a complex task execution system. Your job is to break down a given task into smaller, actionable subtasks.

Please respond with a JSON object in the following format:
{
  "subtasks": [
    {
      "description": "clear, concise description of the subtask",
      "order": 1,
      "estimated_duration_minutes": 5
    }
  ]
}

Guidelines:
- Break the task into 3-10 logical subtasks
- Each subtask should be actionable and specific
- Order the subtasks in the sequence they should be executed
- Estimate reasonable durations for each subtask
- Keep descriptions clear and actionable

Only respond with the JSON, no other text."#.to_string();

        let response = llm_manager
            .chat_with_system_prompt(system_prompt, task.description.clone())
            .await;

        match response {
            Ok(chat_response) => {
                if let Some(choice) = chat_response.choices.first() {
                    let content = &choice.message.content;
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content) {
                        if let Some(subtasks) = parsed.get("subtasks").and_then(|s| s.as_array()) {
                            let mut sub_tasks = Vec::new();
                            for (i, subtask) in subtasks.iter().enumerate() {
                                if let Some(desc) =
                                    subtask.get("description").and_then(|d| d.as_str())
                                {
                                    let mut sub_task = Task::new(desc.to_string(), task.priority);
                                    let mut metadata = serde_json::Map::new();
                                    metadata.insert(
                                        "parent_task_id".to_string(),
                                        serde_json::Value::String(task.id.clone()),
                                    );
                                    metadata.insert(
                                        "step".to_string(),
                                        serde_json::Value::Number(serde_json::Number::from(i + 1)),
                                    );
                                    if let Some(duration) = subtask
                                        .get("estimated_duration_minutes")
                                        .and_then(|d| d.as_u64())
                                    {
                                        metadata.insert(
                                            "estimated_duration_minutes".to_string(),
                                            serde_json::Value::Number(serde_json::Number::from(
                                                duration,
                                            )),
                                        );
                                    }
                                    sub_task.metadata = serde_json::Value::Object(metadata);
                                    sub_tasks.push(sub_task);
                                }
                            }
                            if !sub_tasks.is_empty() {
                                debug!("Task decomposed into {} subtasks (LLM)", sub_tasks.len());
                                return Ok(sub_tasks);
                            }
                        }
                    }
                }
                warn!("LLM decomposition failed, falling back to rule-based");
                self.decompose_task_rule_based(task)
            }
            Err(e) => {
                warn!("LLM decomposition error, falling back to rule-based: {}", e);
                self.decompose_task_rule_based(task)
            }
        }
    }

    pub fn validate_plan(&self, plan: &ExecutionPlan) -> Result<bool> {
        let mut visited = HashSet::new();
        let mut stack = vec![&plan.root_task_id];

        while let Some(node_id) = stack.pop() {
            if visited.contains(node_id) {
                return Err(AetherisError::Planning(
                    "Circular dependency detected in plan".to_string(),
                ));
            }
            visited.insert(node_id);

            if let Some(node) = plan.get_node(node_id) {
                for dep in &node.dependencies {
                    stack.push(&dep.source_task_id);
                }
            }
        }

        Ok(true)
    }
}

#[derive(Serialize, Deserialize)]
struct GraphData {
    nodes: Vec<(String, TaskNode)>,
    edges: Vec<(String, String, TaskDependency)>,
}

impl TaskPlanner {
    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        let mut nodes = Vec::new();
        let mut index_to_id = HashMap::new();

        for (node_id, &node_index) in &self.node_indices {
            if let Some(node) = self.graph.node_weight(node_index) {
                nodes.push((node_id.clone(), node.clone()));
                index_to_id.insert(node_index, node_id.clone());
            }
        }

        let mut edges = Vec::new();
        for edge in self.graph.edge_references() {
            let (source_idx, target_idx) = (edge.source(), edge.target());
            let dependency = edge.weight().clone();

            let source_node_id = index_to_id.get(&source_idx).cloned().ok_or_else(|| {
                AetherisError::Planning("Could not find source node ID".to_string())
            })?;

            let target_node_id = index_to_id.get(&target_idx).cloned().ok_or_else(|| {
                AetherisError::Planning("Could not find target node ID".to_string())
            })?;

            edges.push((source_node_id, target_node_id, dependency));
        }

        let graph_data = GraphData { nodes, edges };
        let file = std::fs::File::create(path)?;
        bincode::serialize_into(file, &graph_data)?;
        Ok(())
    }

    pub fn load(&mut self, path: &std::path::Path) -> Result<()> {
        let file = std::fs::File::open(path)?;
        let graph_data: GraphData = bincode::deserialize_from(file)?;

        self.graph.clear();
        self.node_indices.clear();

        for (node_id, task_node) in graph_data.nodes {
            let node_index = self.graph.add_node(task_node);
            self.node_indices.insert(node_id, node_index);
        }

        for (source_node_id, target_node_id, dependency) in graph_data.edges {
            if let (Some(&source_idx), Some(&target_idx)) = (
                self.node_indices.get(&source_node_id),
                self.node_indices.get(&target_node_id),
            ) {
                self.graph.add_edge(source_idx, target_idx, dependency);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CriticalPathInfo {
    pub task_id: String,
    pub earliest_start_ms: u64,
    pub earliest_finish_ms: u64,
    pub latest_start_ms: u64,
    pub latest_finish_ms: u64,
    pub slack_ms: u64,
    pub is_critical: bool,
}

#[derive(Debug, Clone)]
pub struct CriticalPathResult {
    pub critical_path: Vec<String>,
    pub total_duration_ms: u64,
    pub task_info: Vec<CriticalPathInfo>,
}

impl TaskPlanner {
    pub fn topological_sort(&mut self, plan: &ExecutionPlan) -> Result<Vec<String>> {
        self.ensure_graph_built(plan)?;

        let sorted_nodes = toposort(&self.graph, None).map_err(|_| {
            AetherisError::Planning("Circular dependency detected in the task graph".to_string())
        })?;

        let mut result = Vec::new();
        for node_idx in sorted_nodes {
            if let Some(node) = self.graph.node_weight(node_idx) {
                result.push(node.node_id.clone());
            }
        }

        Ok(result)
    }

    pub fn critical_path(&mut self, plan: &ExecutionPlan) -> Result<CriticalPathResult> {
        self.ensure_graph_built(plan)?;

        let sorted_nodes = toposort(&self.graph, None).map_err(|_| {
            AetherisError::Planning("Circular dependency detected in the task graph".to_string())
        })?;

        let mut earliest_start = HashMap::new();
        let mut earliest_finish = HashMap::new();
        let mut latest_start = HashMap::new();
        let mut latest_finish = HashMap::new();
        let mut slack = HashMap::new();

        for &node_idx in &sorted_nodes {
            if let Some(node) = self.graph.node_weight(node_idx) {
                let mut max_earliest_finish = 0;
                for edge in self
                    .graph
                    .edges_directed(node_idx, petgraph::Direction::Incoming)
                {
                    let pred_idx = edge.source();
                    if let Some(&ef) = earliest_finish.get(&pred_idx) {
                        if ef > max_earliest_finish {
                            max_earliest_finish = ef;
                        }
                    }
                }
                let es = max_earliest_finish;
                let ef = es + node.estimated_duration_ms;
                earliest_start.insert(node_idx, es);
                earliest_finish.insert(node_idx, ef);
            }
        }

        let mut total_duration = 0;
        for &node_idx in &sorted_nodes {
            if let Some(&ef) = earliest_finish.get(&node_idx) {
                if ef > total_duration {
                    total_duration = ef;
                }
            }
        }

        for &node_idx in sorted_nodes.iter().rev() {
            if let Some(node) = self.graph.node_weight(node_idx) {
                let mut min_latest_start = total_duration;
                for edge in self
                    .graph
                    .edges_directed(node_idx, petgraph::Direction::Outgoing)
                {
                    let succ_idx = edge.target();
                    if let Some(&ls) = latest_start.get(&succ_idx) {
                        if ls < min_latest_start {
                            min_latest_start = ls;
                        }
                    }
                }
                let lf = min_latest_start;
                let ls = lf - node.estimated_duration_ms;
                latest_start.insert(node_idx, ls);
                latest_finish.insert(node_idx, lf);
                let s = ls - earliest_start[&node_idx];
                slack.insert(node_idx, s);
            }
        }

        let mut task_info = Vec::new();
        let mut critical_path = Vec::new();

        for &node_idx in &sorted_nodes {
            if let Some(node) = self.graph.node_weight(node_idx) {
                let is_critical = slack[&node_idx] == 0;
                task_info.push(CriticalPathInfo {
                    task_id: node.node_id.clone(),
                    earliest_start_ms: earliest_start[&node_idx],
                    earliest_finish_ms: earliest_finish[&node_idx],
                    latest_start_ms: latest_start[&node_idx],
                    latest_finish_ms: latest_finish[&node_idx],
                    slack_ms: slack[&node_idx],
                    is_critical,
                });
                if is_critical {
                    critical_path.push(node.node_id.clone());
                }
            }
        }

        Ok(CriticalPathResult {
            critical_path,
            total_duration_ms: total_duration,
            task_info,
        })
    }

    pub fn parallel_batches(&mut self, plan: &ExecutionPlan) -> Result<Vec<Vec<String>>> {
        self.ensure_graph_built(plan)?;

        let sorted_nodes = toposort(&self.graph, None).map_err(|_| {
            AetherisError::Planning("Circular dependency detected in the task graph".to_string())
        })?;

        let mut in_degree = HashMap::new();
        let mut node_to_id = HashMap::new();

        for &node_idx in &sorted_nodes {
            if let Some(node) = self.graph.node_weight(node_idx) {
                let degree = self
                    .graph
                    .edges_directed(node_idx, petgraph::Direction::Incoming)
                    .count();
                in_degree.insert(node_idx, degree);
                node_to_id.insert(node_idx, node.node_id.clone());
            }
        }

        let mut batches = Vec::new();
        let mut remaining_degree = in_degree.clone();

        while !remaining_degree.is_empty() {
            let mut current_batch = Vec::new();
            let mut nodes_to_remove = Vec::new();

            for (&node_idx, &degree) in &remaining_degree {
                if degree == 0 {
                    if let Some(node_id) = node_to_id.get(&node_idx) {
                        current_batch.push(node_id.clone());
                    }
                    nodes_to_remove.push(node_idx);
                }
            }

            if current_batch.is_empty() {
                break;
            }

            batches.push(current_batch);

            for node_idx in nodes_to_remove {
                remaining_degree.remove(&node_idx);
                for edge in self
                    .graph
                    .edges_directed(node_idx, petgraph::Direction::Outgoing)
                {
                    let succ_idx = edge.target();
                    if let Some(degree) = remaining_degree.get_mut(&succ_idx) {
                        *degree -= 1;
                    }
                }
            }
        }

        Ok(batches)
    }

    pub fn add_task_to_plan(&mut self, plan: &mut ExecutionPlan, task: Task) -> Result<()> {
        let node = TaskNode::new(task);
        plan.add_node(node);

        self.ensure_graph_built(plan)?;

        Ok(())
    }

    pub fn remove_task_from_plan(&mut self, plan: &mut ExecutionPlan, task_id: &str) -> Result<()> {
        if task_id == plan.root_task_id {
            return Err(AetherisError::Planning(
                "Cannot remove root task from execution plan".to_string(),
            ));
        }

        if let Some(node) = plan.nodes.remove(task_id) {
            for dependent_id in &node.dependents {
                if let Some(dependent_node) = plan.nodes.get_mut(dependent_id) {
                    dependent_node
                        .dependencies
                        .retain(|d| d.source_task_id != task_id);
                }
            }

            for dep in &node.dependencies {
                if let Some(source_node) = plan.nodes.get_mut(&dep.source_task_id) {
                    source_node.dependents.retain(|d| d != task_id);
                }
            }

            plan.dependencies
                .retain(|d| d.source_task_id != task_id && d.target_task_id != task_id);

            plan.updated_at = chrono::Utc::now();

            self.ensure_graph_built(plan)?;
        }

        Ok(())
    }

    fn ensure_graph_built(&mut self, plan: &ExecutionPlan) -> Result<()> {
        if self.graph.node_count() != plan.nodes.len() {
            self.build_dependency_graph_from_plan(plan)?;
        }
        Ok(())
    }

    fn build_dependency_graph_from_plan(&mut self, plan: &ExecutionPlan) -> Result<()> {
        self.graph.clear();
        self.node_indices.clear();

        for node_id in plan.nodes.keys() {
            if let Some(task_node) = plan.nodes.get(node_id) {
                let node_index = self.graph.add_node(task_node.clone());
                self.node_indices.insert(node_id.clone(), node_index);
            }
        }

        for dependency in &plan.dependencies {
            if let (Some(&source_idx), Some(&target_idx)) = (
                self.node_indices.get(&dependency.source_task_id),
                self.node_indices.get(&dependency.target_task_id),
            ) {
                self.graph
                    .add_edge(source_idx, target_idx, dependency.clone());
            }
        }

        Ok(())
    }
}

impl Default for TaskPlanner {
    fn default() -> Self {
        Self::new()
    }
}
