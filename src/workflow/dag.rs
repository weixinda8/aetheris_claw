use crate::Result;
use crate::workflow::{
    ExecutionStrategy, NodeExecutionContext, Workflow, WorkflowEdge, WorkflowExecutionContext,
    WorkflowNode, WorkflowStatus,
};
use async_trait::async_trait;
use petgraph::algo::is_cyclic_directed;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Topo;
use std::collections::{HashMap, HashSet};

pub struct DAGValidator;

impl DAGValidator {
    pub fn validate(workflow: &Workflow) -> Result<bool> {
        let mut graph = DiGraph::new();
        let mut node_indices: HashMap<String, NodeIndex> = HashMap::new();

        for node in &workflow.nodes {
            let idx = graph.add_node(node.id.clone());
            node_indices.insert(node.id.clone(), idx);
        }

        for edge in &workflow.edges {
            if let (Some(&source_idx), Some(&target_idx)) = (
                node_indices.get(&edge.source_node_id),
                node_indices.get(&edge.target_node_id),
            ) {
                graph.add_edge(source_idx, target_idx, ());
            }
        }

        Ok(!is_cyclic_directed(&graph))
    }
}

pub struct DAGTopologicalSorter;

impl DAGTopologicalSorter {
    pub fn sort(workflow: &Workflow) -> Result<Vec<String>> {
        let mut graph = DiGraph::new();
        let mut node_indices: HashMap<String, NodeIndex> = HashMap::new();
        let mut index_to_node: HashMap<NodeIndex, String> = HashMap::new();

        for node in &workflow.nodes {
            let idx = graph.add_node(node.id.clone());
            node_indices.insert(node.id.clone(), idx);
            index_to_node.insert(idx, node.id.clone());
        }

        for edge in &workflow.edges {
            if let (Some(&source_idx), Some(&target_idx)) = (
                node_indices.get(&edge.source_node_id),
                node_indices.get(&edge.target_node_id),
            ) {
                graph.add_edge(source_idx, target_idx, ());
            }
        }

        let mut topo = Topo::new(&graph);
        let mut sorted = Vec::new();

        for node_idx in topo {
            if let Some(node_id) = index_to_node.get(&node_idx) {
                sorted.push(node_id.clone());
            }
        }

        Ok(sorted)
    }
}

pub struct DAGExecutor {
    strategy: ExecutionStrategy,
}

impl DAGExecutor {
    pub fn new(strategy: ExecutionStrategy) -> Self {
        Self { strategy }
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

    pub async fn execute_serial(
        &self,
        workflow: &Workflow,
        _context: &mut WorkflowExecutionContext,
    ) -> Result<()> {
        let sorted_nodes = DAGTopologicalSorter::sort(workflow)?;

        for node_id in sorted_nodes {
            self.execute_node(workflow, node_id, _context).await?;
        }

        Ok(())
    }

    pub async fn execute_parallel(
        &self,
        workflow: &Workflow,
        _context: &mut WorkflowExecutionContext,
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
                let mut context_clone = _context.clone();

                handles.push(tokio::spawn(async move {
                    let mut executor = DAGExecutor::new(ExecutionStrategy::Serial);
                    let result = executor
                        .execute_node(&workflow_clone, node_id_clone, &mut context_clone)
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
                        _context.node_contexts.extend(updated_context.node_contexts);
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(())
    }

    async fn execute_node(
        &self,
        _workflow: &Workflow,
        node_id: String,
        context: &mut WorkflowExecutionContext,
    ) -> Result<()> {
        let mut node_context = NodeExecutionContext::new(node_id.clone());
        node_context.status = WorkflowStatus::Running;
        node_context.start_time = Some(chrono::Utc::now());

        context.node_contexts.insert(node_id.clone(), node_context);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        if let Some(node_context) = context.node_contexts.get_mut(&node_id) {
            let mut updated_context = node_context.clone();
            updated_context.status = WorkflowStatus::Completed;
            updated_context.end_time = Some(chrono::Utc::now());
            context.node_contexts.insert(node_id, updated_context);
        }

        Ok(())
    }
}

#[async_trait]
impl crate::workflow::WorkflowExecutor for DAGExecutor {
    async fn execute(&self, workflow: &Workflow) -> Result<WorkflowExecutionContext> {
        if !DAGValidator::validate(workflow)? {
            return Err(crate::AetherisError::Validation(
                "Workflow contains cycles".to_string(),
            ));
        }

        let mut context = WorkflowExecutionContext::new(workflow.id.clone());

        match self.strategy {
            ExecutionStrategy::Serial => self.execute_serial(workflow, &mut context).await?,
            ExecutionStrategy::Parallel => self.execute_parallel(workflow, &mut context).await?,
        }

        Ok(context)
    }

    async fn pause(&self, _execution_id: &str) -> Result<()> {
        Ok(())
    }

    async fn resume(&self, _execution_id: &str) -> Result<()> {
        Ok(())
    }

    async fn cancel(&self, _execution_id: &str) -> Result<()> {
        Ok(())
    }

    async fn get_status(&self, _execution_id: &str) -> Result<WorkflowStatus> {
        Ok(WorkflowStatus::Completed)
    }
}
