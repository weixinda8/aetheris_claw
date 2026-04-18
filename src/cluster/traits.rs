use crate::cluster::types::*;
use crate::utils::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait ClusterNode: Send + Sync {
    fn node_id(&self) -> &str;
    fn config(&self) -> &ClusterConfig;

    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;

    fn role(&self) -> NodeRole;
    fn is_leader(&self) -> bool;
    fn leader_id(&self) -> Option<String>;
    fn peers(&self) -> Vec<NodeInfo>;
    fn metrics(&self) -> ClusterMetrics;
}

#[async_trait]
pub trait RaftConsensus: Send + Sync {
    async fn submit_command(&mut self, command: Vec<u8>) -> Result<u64>;
    async fn get_state(&self) -> RaftState;
    async fn get_log_entry(&self, index: u64) -> Option<RaftLogEntry>;
    async fn get_last_log_index(&self) -> u64;
    async fn get_last_log_term(&self) -> u64;
}

#[async_trait]
pub trait LoadBalancer: Send + Sync {
    fn config(&self) -> &LoadBalancingConfig;

    async fn select_node(&self, task_id: &str, nodes: &[NodeInfo]) -> Result<Option<String>>;
    async fn rebalance(
        &self,
        tasks: &[String],
        nodes: &[NodeInfo],
    ) -> Result<HashMap<String, Vec<String>>>;
    async fn report_task_completion(&mut self, node_id: &str, task_id: &str) -> Result<()>;
    async fn report_task_start(&mut self, node_id: &str, task_id: &str) -> Result<()>;
}

pub struct SimpleLoadBalancer {
    config: LoadBalancingConfig,
    node_task_counts: Arc<dashmap::DashMap<String, usize>>,
}

impl SimpleLoadBalancer {
    pub fn new(config: LoadBalancingConfig) -> Self {
        Self {
            config,
            node_task_counts: Arc::new(dashmap::DashMap::new()),
        }
    }
}

#[async_trait]
impl LoadBalancer for SimpleLoadBalancer {
    fn config(&self) -> &LoadBalancingConfig {
        &self.config
    }

    async fn select_node(&self, _task_id: &str, nodes: &[NodeInfo]) -> Result<Option<String>> {
        let alive_nodes: Vec<_> = nodes.iter().filter(|n| n.is_alive).collect();

        if alive_nodes.is_empty() {
            return Ok(None);
        }

        let selected = match self.config.strategy {
            LoadBalancingStrategy::RoundRobin => {
                let mut min_index = 0;
                let mut min_count = usize::MAX;

                for (i, node) in alive_nodes.iter().enumerate() {
                    let count = self
                        .node_task_counts
                        .get(&node.node_id)
                        .map(|c| *c)
                        .unwrap_or(0);
                    if count < min_count {
                        min_count = count;
                        min_index = i;
                    }
                }

                alive_nodes[min_index].node_id.clone()
            }
            LoadBalancingStrategy::LeastLoaded => {
                let mut min_count = usize::MAX;
                let mut selected = None;

                for node in alive_nodes.iter() {
                    let count = self
                        .node_task_counts
                        .get(&node.node_id)
                        .map(|c| *c)
                        .unwrap_or(0);
                    if count < min_count {
                        min_count = count;
                        selected = Some(node.node_id.clone());
                    }
                }

                selected.unwrap_or_else(|| alive_nodes[0].node_id.clone())
            }
            _ => alive_nodes[0].node_id.clone(),
        };

        Ok(Some(selected))
    }

    async fn rebalance(
        &self,
        _tasks: &[String],
        _nodes: &[NodeInfo],
    ) -> Result<HashMap<String, Vec<String>>> {
        Ok(HashMap::new())
    }

    async fn report_task_completion(&mut self, node_id: &str, _task_id: &str) -> Result<()> {
        self.node_task_counts
            .entry(node_id.to_string())
            .and_modify(|count| *count = count.saturating_sub(1))
            .or_insert(0);
        Ok(())
    }

    async fn report_task_start(&mut self, node_id: &str, _task_id: &str) -> Result<()> {
        self.node_task_counts
            .entry(node_id.to_string())
            .and_modify(|count| *count += 1)
            .or_insert(1);
        Ok(())
    }
}
