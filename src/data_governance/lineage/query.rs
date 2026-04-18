use crate::data_governance::{LineageNode, LineageNodeId, LineageStore};
use std::collections::{HashMap, HashSet};

pub struct LineageQueryEngine;

impl LineageQueryEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn query_upstream(
        &self,
        store: &LineageStore,
        node_id: &LineageNodeId,
        depth: Option<usize>,
    ) -> Vec<LineageNode> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();

        self.traverse_upstream_recursive(store, node_id, &mut result, &mut visited, depth, 0);

        result
    }

    fn traverse_upstream_recursive(
        &self,
        store: &LineageStore,
        node_id: &LineageNodeId,
        result: &mut Vec<LineageNode>,
        visited: &mut HashSet<LineageNodeId>,
        max_depth: Option<usize>,
        current_depth: usize,
    ) {
        if visited.contains(node_id) {
            return;
        }

        if let Some(max_d) = max_depth {
            if current_depth > max_d {
                return;
            }
        }

        visited.insert(node_id.clone());

        for edge in store.get_all_edges() {
            if &edge.target_node_id == node_id {
                if let Some(source_node) = store.get_node(&edge.source_node_id) {
                    if !visited.contains(&source_node.id) {
                        result.push(source_node.clone());
                        self.traverse_upstream_recursive(
                            store,
                            &source_node.id,
                            result,
                            visited,
                            max_depth,
                            current_depth + 1,
                        );
                    }
                }
            }
        }
    }

    pub fn query_downstream(
        &self,
        store: &LineageStore,
        node_id: &LineageNodeId,
        depth: Option<usize>,
    ) -> Vec<LineageNode> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();

        self.traverse_downstream_recursive(store, node_id, &mut result, &mut visited, depth, 0);

        result
    }

    fn traverse_downstream_recursive(
        &self,
        store: &LineageStore,
        node_id: &LineageNodeId,
        result: &mut Vec<LineageNode>,
        visited: &mut HashSet<LineageNodeId>,
        max_depth: Option<usize>,
        current_depth: usize,
    ) {
        if visited.contains(node_id) {
            return;
        }

        if let Some(max_d) = max_depth {
            if current_depth > max_d {
                return;
            }
        }

        visited.insert(node_id.clone());

        for edge in store.get_all_edges() {
            if &edge.source_node_id == node_id {
                if let Some(target_node) = store.get_node(&edge.target_node_id) {
                    if !visited.contains(&target_node.id) {
                        result.push(target_node.clone());
                        self.traverse_downstream_recursive(
                            store,
                            &target_node.id,
                            result,
                            visited,
                            max_depth,
                            current_depth + 1,
                        );
                    }
                }
            }
        }
    }

    pub fn find_path(
        &self,
        store: &LineageStore,
        source_id: &LineageNodeId,
        target_id: &LineageNodeId,
    ) -> Option<Vec<LineageNodeId>> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        if self.dfs_find_path(store, source_id, target_id, &mut path, &mut visited) {
            return Some(path);
        }

        None
    }

    fn dfs_find_path(
        &self,
        store: &LineageStore,
        current_id: &LineageNodeId,
        target_id: &LineageNodeId,
        path: &mut Vec<LineageNodeId>,
        visited: &mut HashSet<LineageNodeId>,
    ) -> bool {
        if visited.contains(current_id) {
            return false;
        }

        visited.insert(current_id.clone());
        path.push(current_id.clone());

        if current_id == target_id {
            return true;
        }

        for edge in store.get_all_edges() {
            if &edge.source_node_id == current_id
                && self.dfs_find_path(store, &edge.target_node_id, target_id, path, visited) {
                    return true;
                }
        }

        path.pop();
        false
    }

    pub fn impact_analysis(
        &self,
        store: &LineageStore,
        node_id: &LineageNodeId,
    ) -> ImpactAnalysisResult {
        let downstream_nodes = self.query_downstream(store, node_id, None);

        let mut node_type_counts = HashMap::new();
        for node in &downstream_nodes {
            *node_type_counts
                .entry(format!("{:?}", node.node_type))
                .or_insert(0) += 1;
        }

        ImpactAnalysisResult {
            affected_nodes: downstream_nodes.len(),
            node_type_counts,
            affected_nodes_list: downstream_nodes,
        }
    }
}

impl Default for LineageQueryEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImpactAnalysisResult {
    pub affected_nodes: usize,
    pub node_type_counts: HashMap<String, usize>,
    pub affected_nodes_list: Vec<LineageNode>,
}
