pub mod collector;
pub mod graph;
pub mod query;
pub mod visualization;

use crate::data_governance::{
    DataLineage, LineageEdge, LineageEdgeId, LineageNode, LineageNodeId, LineageTracker,
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

pub use collector::LineageCollector;
pub use graph::LineageGraphBuilder;
pub use query::LineageQueryEngine;
pub use visualization::{LineageVisualizationData, VisualizationFormat};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageRecord {
    pub id: String,
    pub record_type: LineageRecordType,
    pub node: Option<LineageNode>,
    pub edge: Option<LineageEdge>,
    pub timestamp: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LineageRecordType {
    NodeCreated,
    NodeUpdated,
    EdgeCreated,
}

impl LineageRecord {
    pub fn new_node_record(node: LineageNode, record_type: LineageRecordType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            record_type,
            node: Some(node),
            edge: None,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
        }
    }

    pub fn new_edge_record(edge: LineageEdge) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            record_type: LineageRecordType::EdgeCreated,
            node: None,
            edge: Some(edge),
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredLineageData {
    nodes: Vec<LineageNode>,
    edges: Vec<LineageEdge>,
    records: Vec<LineageRecord>,
}

pub struct LineageStore {
    nodes: DashMap<LineageNodeId, LineageNode>,
    edges: DashMap<LineageEdgeId, LineageEdge>,
    records: Arc<RwLock<Vec<LineageRecord>>>,
    query_engine: LineageQueryEngine,
    storage_path: Option<PathBuf>,
}

impl LineageStore {
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
            edges: DashMap::new(),
            records: Arc::new(RwLock::new(Vec::new())),
            query_engine: LineageQueryEngine::new(),
            storage_path: None,
        }
    }

    pub fn with_storage_path(storage_path: PathBuf) -> Self {
        let mut store = Self::new();
        store.storage_path = Some(storage_path);
        store
    }

    pub fn set_storage_path(&mut self, storage_path: PathBuf) {
        self.storage_path = Some(storage_path);
    }

    pub fn save_to_disk(&self) -> crate::utils::Result<()> {
        if let Some(path) = &self.storage_path {
            let stored_data = StoredLineageData {
                nodes: self.get_all_nodes(),
                edges: self.get_all_edges(),
                records: self.get_records(),
            };

            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let file = std::fs::File::create(path)?;
            serde_json::to_writer_pretty(file, &stored_data)?;
        }
        Ok(())
    }

    pub fn load_from_disk(&self) -> crate::utils::Result<bool> {
        if let Some(path) = &self.storage_path {
            if path.exists() {
                let file = std::fs::File::open(path)?;
                let stored_data: StoredLineageData = serde_json::from_reader(file)?;

                for node in stored_data.nodes {
                    self.nodes.insert(node.id.clone(), node);
                }

                for edge in stored_data.edges {
                    self.edges.insert(edge.id.clone(), edge);
                }

                *self.records.write() = stored_data.records;

                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn store_node(&self, node: LineageNode) -> crate::utils::Result<LineageNodeId> {
        let node_id = node.id.clone();
        let record = LineageRecord::new_node_record(node.clone(), LineageRecordType::NodeCreated);
        self.nodes.insert(node_id.clone(), node);
        self.records.write().push(record);
        Ok(node_id)
    }

    pub fn store_edge(&self, edge: LineageEdge) -> crate::utils::Result<LineageEdgeId> {
        let edge_id = edge.id.clone();
        let record = LineageRecord::new_edge_record(edge.clone());
        self.edges.insert(edge_id.clone(), edge);
        self.records.write().push(record);
        Ok(edge_id)
    }

    pub fn get_node(&self, node_id: &LineageNodeId) -> Option<LineageNode> {
        self.nodes.get(node_id).map(|n| n.clone())
    }

    pub fn get_all_nodes(&self) -> Vec<LineageNode> {
        self.nodes.iter().map(|n| n.clone()).collect()
    }

    pub fn get_all_edges(&self) -> Vec<LineageEdge> {
        self.edges.iter().map(|e| e.clone()).collect()
    }

    pub fn get_records(&self) -> Vec<LineageRecord> {
        self.records.read().clone()
    }

    pub fn build_lineage(&self, node_id: &LineageNodeId) -> DataLineage {
        let mut lineage = DataLineage::new();

        if let Some(node) = self.get_node(node_id) {
            lineage.add_node(node);
        }

        let upstream_nodes = self.query_engine.query_upstream(self, node_id, None);
        for node in upstream_nodes {
            lineage.add_node(node);
        }

        let downstream_nodes = self.query_engine.query_downstream(self, node_id, None);
        for node in downstream_nodes {
            lineage.add_node(node);
        }

        for edge in self.get_all_edges() {
            if lineage.nodes.contains_key(&edge.source_node_id)
                && lineage.nodes.contains_key(&edge.target_node_id)
            {
                lineage.add_edge(edge);
            }
        }

        lineage
    }

    pub fn build_full_lineage(&self) -> DataLineage {
        let mut lineage = DataLineage::new();

        for node in self.get_all_nodes() {
            lineage.add_node(node);
        }

        for edge in self.get_all_edges() {
            lineage.add_edge(edge);
        }

        lineage
    }

    pub fn export_to_json(&self) -> crate::utils::Result<String> {
        let full_lineage = self.build_full_lineage();
        Ok(serde_json::to_string_pretty(&full_lineage)?)
    }

    pub fn export_to_graphml(&self) -> crate::utils::Result<String> {
        let full_lineage = self.build_full_lineage();
        let mut graphml = String::new();

        graphml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        graphml.push_str("<graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\">\n");
        graphml.push_str(
            "  <key id=\"name\" for=\"node\" attr.name=\"name\" attr.type=\"string\"/>\n",
        );
        graphml.push_str(
            "  <key id=\"type\" for=\"node\" attr.name=\"type\" attr.type=\"string\"/>\n",
        );
        graphml.push_str(
            "  <key id=\"label\" for=\"edge\" attr.name=\"label\" attr.type=\"string\"/>\n",
        );
        graphml.push_str("  <graph id=\"lineage\" edgedefault=\"directed\">\n");

        for (node_id, node) in &full_lineage.nodes {
            let node_type_str = format!("{:?}", node.node_type);
            graphml.push_str(&format!("    <node id=\"{}\">\n", node_id.0));
            graphml.push_str(&format!("      <data key=\"name\">{}</data>\n", node.name));
            graphml.push_str(&format!(
                "      <data key=\"type\">{}</data>\n",
                node_type_str
            ));
            graphml.push_str("    </node>\n");
        }

        for (i, edge) in full_lineage.edges.iter().enumerate() {
            let edge_type_str = format!("{:?}", edge.edge_type);
            graphml.push_str(&format!(
                "    <edge id=\"e{}\" source=\"{}\" target=\"{}\">\n",
                i, edge.source_node_id.0, edge.target_node_id.0
            ));
            graphml.push_str(&format!(
                "      <data key=\"label\">{}</data>\n",
                edge_type_str
            ));
            graphml.push_str("    </edge>\n");
        }

        graphml.push_str("  </graph>\n");
        graphml.push_str("</graphml>\n");

        Ok(graphml)
    }
}

impl Default for LineageStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LineageTracker for LineageStore {
    async fn record_node(&self, node: LineageNode) -> crate::utils::Result<LineageNodeId> {
        self.store_node(node)
    }

    async fn record_edge(&self, edge: LineageEdge) -> crate::utils::Result<LineageEdgeId> {
        self.store_edge(edge)
    }

    async fn get_node(&self, node_id: &LineageNodeId) -> crate::utils::Result<Option<LineageNode>> {
        Ok(self.get_node(node_id))
    }

    async fn get_lineage(&self, node_id: &LineageNodeId) -> crate::utils::Result<DataLineage> {
        Ok(self.build_lineage(node_id))
    }

    async fn query_upstream(
        &self,
        node_id: &LineageNodeId,
        depth: Option<usize>,
    ) -> crate::utils::Result<Vec<LineageNode>> {
        Ok(self.query_engine.query_upstream(self, node_id, depth))
    }

    async fn query_downstream(
        &self,
        node_id: &LineageNodeId,
        depth: Option<usize>,
    ) -> crate::utils::Result<Vec<LineageNode>> {
        Ok(self.query_engine.query_downstream(self, node_id, depth))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_governance::{LineageEdge, LineageEdgeType, LineageNode, LineageNodeType};
    use tempfile::tempdir;

    #[test]
    fn test_lineage_store_new() {
        let store = LineageStore::new();
        assert!(store.get_all_nodes().is_empty());
        assert!(store.get_all_edges().is_empty());
        assert!(store.get_records().is_empty());
    }

    #[test]
    fn test_lineage_store_default() {
        let store = LineageStore::default();
        assert!(store.get_all_nodes().is_empty());
    }

    #[test]
    fn test_lineage_store_store_node() {
        let store = LineageStore::new();
        let node = LineageNode::new(LineageNodeType::Table, "test_table".to_string());
        let node_id = node.id.clone();

        let result = store.store_node(node);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), node_id);

        let retrieved = store.get_node(&node_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test_table");
    }

    #[test]
    fn test_lineage_store_store_edge() {
        let store = LineageStore::new();
        let source_id = LineageNodeId::new();
        let target_id = LineageNodeId::new();
        let edge = LineageEdge::new(LineageEdgeType::ReadsFrom, source_id, target_id);
        let edge_id = edge.id.clone();

        let result = store.store_edge(edge);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), edge_id);

        let edges = store.get_all_edges();
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_lineage_store_get_all_nodes() {
        let store = LineageStore::new();

        let node1 = LineageNode::new(LineageNodeType::Table, "table1".to_string());
        let node2 = LineageNode::new(LineageNodeType::Column, "column1".to_string());

        store.store_node(node1).unwrap();
        store.store_node(node2).unwrap();

        let nodes = store.get_all_nodes();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_lineage_store_get_all_edges() {
        let store = LineageStore::new();

        let source_id1 = LineageNodeId::new();
        let target_id1 = LineageNodeId::new();
        let edge1 = LineageEdge::new(LineageEdgeType::ReadsFrom, source_id1, target_id1);

        let source_id2 = LineageNodeId::new();
        let target_id2 = LineageNodeId::new();
        let edge2 = LineageEdge::new(LineageEdgeType::WritesTo, source_id2, target_id2);

        store.store_edge(edge1).unwrap();
        store.store_edge(edge2).unwrap();

        let edges = store.get_all_edges();
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_lineage_store_get_records() {
        let store = LineageStore::new();
        let node = LineageNode::new(LineageNodeType::Table, "test".to_string());

        store.store_node(node).unwrap();

        let records = store.get_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record_type, LineageRecordType::NodeCreated);
    }

    #[test]
    fn test_lineage_record_new_node_record() {
        let node = LineageNode::new(LineageNodeType::Table, "test".to_string());
        let record = LineageRecord::new_node_record(node, LineageRecordType::NodeCreated);

        assert_eq!(record.record_type, LineageRecordType::NodeCreated);
        assert!(record.node.is_some());
        assert!(record.edge.is_none());
    }

    #[test]
    fn test_lineage_record_new_edge_record() {
        let source_id = LineageNodeId::new();
        let target_id = LineageNodeId::new();
        let edge = LineageEdge::new(LineageEdgeType::ReadsFrom, source_id, target_id);
        let record = LineageRecord::new_edge_record(edge);

        assert_eq!(record.record_type, LineageRecordType::EdgeCreated);
        assert!(record.node.is_none());
        assert!(record.edge.is_some());
    }

    #[test]
    fn test_lineage_record_type_equality() {
        assert_eq!(
            LineageRecordType::NodeCreated,
            LineageRecordType::NodeCreated
        );
        assert_eq!(
            LineageRecordType::NodeUpdated,
            LineageRecordType::NodeUpdated
        );
        assert_eq!(
            LineageRecordType::EdgeCreated,
            LineageRecordType::EdgeCreated
        );
    }

    #[test]
    fn test_lineage_query_engine_new() {
        let engine = LineageQueryEngine::new();
        assert!(true);
    }

    #[test]
    fn test_lineage_query_engine_default() {
        let engine = LineageQueryEngine::default();
        assert!(true);
    }

    #[test]
    fn test_lineage_store_persistence() {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("lineage.json");

        let store = LineageStore::with_storage_path(storage_path.clone());

        let node1 = LineageNode::new(LineageNodeType::Table, "source_table".to_string());
        let node2 = LineageNode::new(LineageNodeType::Table, "target_table".to_string());
        let edge = LineageEdge::new(
            LineageEdgeType::TransformsTo,
            node1.id.clone(),
            node2.id.clone(),
        );

        store.store_node(node1).unwrap();
        store.store_node(node2).unwrap();
        store.store_edge(edge).unwrap();

        store.save_to_disk().unwrap();

        let new_store = LineageStore::with_storage_path(storage_path);
        let loaded = new_store.load_from_disk().unwrap();
        assert!(loaded);

        let nodes = new_store.get_all_nodes();
        assert_eq!(nodes.len(), 2);

        let edges = new_store.get_all_edges();
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_lineage_collector_integration() {
        let store = Arc::new(LineageStore::new());
        let mut collector = LineageCollector::with_store(store.clone());

        collector.set_auto_flush(false);

        let source_id = collector.collect_data_source(
            "test_source".to_string(),
            "database".to_string(),
            HashMap::new(),
        );

        let transform_id = collector.collect_transform(
            "test_transform".to_string(),
            "filter".to_string(),
            vec![source_id.clone()],
            HashMap::new(),
        );

        let sink_id = collector.collect_data_sink(
            "test_sink".to_string(),
            "database".to_string(),
            vec![transform_id.clone()],
            HashMap::new(),
        );

        collector.flush().unwrap();

        let nodes = store.get_all_nodes();
        assert_eq!(nodes.len(), 3);

        let edges = store.get_all_edges();
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_lineage_export_to_json() {
        let store = LineageStore::new();

        let node1 = LineageNode::new(LineageNodeType::Table, "source_table".to_string());
        let node2 = LineageNode::new(LineageNodeType::Table, "target_table".to_string());
        let edge = LineageEdge::new(
            LineageEdgeType::TransformsTo,
            node1.id.clone(),
            node2.id.clone(),
        );

        store.store_node(node1).unwrap();
        store.store_node(node2).unwrap();
        store.store_edge(edge).unwrap();

        let json_export = store.export_to_json().unwrap();
        assert!(!json_export.is_empty());
        assert!(json_export.contains("source_table"));
        assert!(json_export.contains("target_table"));
    }

    #[test]
    fn test_lineage_export_to_graphml() {
        let store = LineageStore::new();

        let node1 = LineageNode::new(LineageNodeType::Table, "source_table".to_string());
        let node2 = LineageNode::new(LineageNodeType::Table, "target_table".to_string());
        let edge = LineageEdge::new(
            LineageEdgeType::TransformsTo,
            node1.id.clone(),
            node2.id.clone(),
        );

        store.store_node(node1).unwrap();
        store.store_node(node2).unwrap();
        store.store_edge(edge).unwrap();

        let graphml_export = store.export_to_graphml().unwrap();
        assert!(!graphml_export.is_empty());
        assert!(graphml_export.contains("<graphml"));
        assert!(graphml_export.contains("<node"));
        assert!(graphml_export.contains("<edge"));
    }

    #[test]
    fn test_lineage_collector_context_stack() {
        let store = Arc::new(LineageStore::new());
        let collector = LineageCollector::with_store(store.clone());

        let context_id = LineageNodeId::new();
        collector.push_context(context_id.clone());

        let current_context = collector.current_context();
        assert!(current_context.is_some());
        assert_eq!(current_context.unwrap(), context_id);

        let popped_context = collector.pop_context();
        assert!(popped_context.is_some());
        assert_eq!(popped_context.unwrap(), context_id);

        assert!(collector.current_context().is_none());
    }

    #[test]
    fn test_lineage_collector_auto_flush() {
        let store = Arc::new(LineageStore::new());
        let mut collector = LineageCollector::with_store(store.clone());

        collector.set_auto_flush(true);

        let source_id = collector.collect_data_source(
            "auto_flush_test".to_string(),
            "test".to_string(),
            HashMap::new(),
        );

        let nodes = store.get_all_nodes();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id, source_id);
    }

    #[test]
    fn test_lineage_collector_node_caching() {
        let store = Arc::new(LineageStore::new());
        let mut collector = LineageCollector::with_store(store.clone());

        let name = "cached_node".to_string();
        let id1 = collector.collect_data_source(name.clone(), "test".to_string(), HashMap::new());

        let id2 = collector.collect_data_source(name, "test".to_string(), HashMap::new());

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_build_full_lineage() {
        let store = LineageStore::new();

        let node1 = LineageNode::new(LineageNodeType::DataSource, "source".to_string());
        let node2 = LineageNode::new(LineageNodeType::DataTransform, "transform".to_string());
        let node3 = LineageNode::new(LineageNodeType::DataSink, "sink".to_string());

        let edge1 = LineageEdge::new(
            LineageEdgeType::ReadsFrom,
            node2.id.clone(),
            node1.id.clone(),
        );
        let edge2 = LineageEdge::new(
            LineageEdgeType::WritesTo,
            node2.id.clone(),
            node3.id.clone(),
        );

        store.store_node(node1).unwrap();
        store.store_node(node2).unwrap();
        store.store_node(node3).unwrap();
        store.store_edge(edge1).unwrap();
        store.store_edge(edge2).unwrap();

        let full_lineage = store.build_full_lineage();
        assert_eq!(full_lineage.nodes.len(), 3);
        assert_eq!(full_lineage.edges.len(), 2);
    }
}
