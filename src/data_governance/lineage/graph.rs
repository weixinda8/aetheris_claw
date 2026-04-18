use crate::data_governance::{DataLineage, LineageEdge, LineageNode, LineageNodeId};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Bfs;
use std::collections::HashMap;

pub struct LineageGraphBuilder {
    graph: DiGraph<LineageNode, LineageEdge>,
    node_indices: HashMap<LineageNodeId, NodeIndex>,
}

impl LineageGraphBuilder {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
        }
    }

    pub fn from_records(nodes: Vec<LineageNode>, edges: Vec<LineageEdge>) -> Self {
        let mut builder = Self::new();

        for node in nodes {
            builder.add_node(node);
        }

        for edge in edges {
            builder.add_edge(edge);
        }

        builder
    }

    pub fn add_node(&mut self, node: LineageNode) -> NodeIndex {
        let node_id = node.id.clone();
        if let Some(&idx) = self.node_indices.get(&node_id) {
            return idx;
        }

        let idx = self.graph.add_node(node);
        self.node_indices.insert(node_id, idx);
        idx
    }

    pub fn add_edge(&mut self, edge: LineageEdge) {
        if let (Some(&source_idx), Some(&target_idx)) = (
            self.node_indices.get(&edge.source_node_id),
            self.node_indices.get(&edge.target_node_id),
        ) {
            self.graph.add_edge(source_idx, target_idx, edge);
        }
    }

    pub fn build(&self) -> DataLineage {
        let mut lineage = DataLineage::new();

        for node in self.graph.node_weights() {
            lineage.add_node(node.clone());
        }

        for edge in self.graph.edge_weights() {
            lineage.add_edge(edge.clone());
        }

        lineage
    }

    pub fn traverse_upstream(
        &self,
        start_node_id: &LineageNodeId,
        max_depth: Option<usize>,
    ) -> Vec<LineageNode> {
        let mut result = Vec::new();

        if let Some(&start_idx) = self.node_indices.get(start_node_id) {
            let mut visited = HashMap::new();
            let mut stack = vec![(start_idx, 0)];

            while let Some((current_idx, depth)) = stack.pop() {
                if visited.contains_key(&current_idx) {
                    continue;
                }

                if let Some(max_d) = max_depth {
                    if depth > max_d {
                        continue;
                    }
                }

                visited.insert(current_idx, true);

                if current_idx != start_idx {
                    if let Some(node) = self.graph.node_weight(current_idx) {
                        result.push(node.clone());
                    }
                }

                for neighbor in self
                    .graph
                    .neighbors_directed(current_idx, petgraph::Direction::Incoming)
                {
                    stack.push((neighbor, depth + 1));
                }
            }
        }

        result
    }

    pub fn traverse_downstream(
        &self,
        start_node_id: &LineageNodeId,
        max_depth: Option<usize>,
    ) -> Vec<LineageNode> {
        let mut result = Vec::new();

        if let Some(&start_idx) = self.node_indices.get(start_node_id) {
            let mut bfs = Bfs::new(&self.graph, start_idx);
            let mut depth_map = HashMap::new();
            depth_map.insert(start_idx, 0);

            while let Some(nx) = bfs.next(&self.graph) {
                let depth = *depth_map.get(&nx).unwrap_or(&0);

                if let Some(max_d) = max_depth {
                    if depth > max_d {
                        continue;
                    }
                }

                if nx != start_idx {
                    if let Some(node) = self.graph.node_weight(nx) {
                        result.push(node.clone());
                    }
                }

                for neighbor in self.graph.neighbors(nx) {
                    depth_map.entry(neighbor).or_insert(depth + 1);
                }
            }
        }

        result
    }

    pub fn find_path(
        &self,
        source_id: &LineageNodeId,
        target_id: &LineageNodeId,
    ) -> Option<Vec<LineageNodeId>> {
        use petgraph::algo::has_path_connecting;

        if let (Some(&source_idx), Some(&target_idx)) = (
            self.node_indices.get(source_id),
            self.node_indices.get(target_id),
        ) {
            if has_path_connecting(&self.graph, source_idx, target_idx, None) {
                let mut path = Vec::new();
                let mut visited = HashMap::new();

                if self.dfs_find_path(source_idx, target_idx, &mut path, &mut visited) {
                    let mut node_ids = Vec::new();
                    for idx in path {
                        if let Some(node) = self.graph.node_weight(idx) {
                            node_ids.push(node.id.clone());
                        }
                    }
                    return Some(node_ids);
                }
            }
        }

        None
    }

    fn dfs_find_path(
        &self,
        current: NodeIndex,
        target: NodeIndex,
        path: &mut Vec<NodeIndex>,
        visited: &mut HashMap<NodeIndex, bool>,
    ) -> bool {
        if visited.contains_key(&current) {
            return false;
        }

        visited.insert(current, true);
        path.push(current);

        if current == target {
            return true;
        }

        for neighbor in self.graph.neighbors(current) {
            if self.dfs_find_path(neighbor, target, path, visited) {
                return true;
            }
        }

        path.pop();
        false
    }

    pub fn get_graph(&self) -> &DiGraph<LineageNode, LineageEdge> {
        &self.graph
    }
}

impl Default for LineageGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}
