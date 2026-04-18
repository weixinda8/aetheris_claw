pub mod classification;
pub mod compliance_reporting;
pub mod lineage;
pub mod masking;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct LineageNodeId(pub String);

impl LineageNodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl Default for LineageNodeId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LineageNodeType {
    DataSource,
    DataTransform,
    DataSink,
    Table,
    Column,
    File,
    Api,
    Agent,
    Task,
    Workflow,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub id: LineageNodeId,
    pub node_type: LineageNodeType,
    pub name: String,
    pub description: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LineageNode {
    pub fn new(node_type: LineageNodeType, name: String) -> Self {
        let now = Utc::now();
        Self {
            id: LineageNodeId::new(),
            node_type,
            name,
            description: None,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct LineageEdgeId(pub String);

impl LineageEdgeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl Default for LineageEdgeId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LineageEdgeType {
    ReadsFrom,
    WritesTo,
    TransformsTo,
    References,
    DependsOn,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    pub id: LineageEdgeId,
    pub edge_type: LineageEdgeType,
    pub source_node_id: LineageNodeId,
    pub target_node_id: LineageNodeId,
    pub description: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl LineageEdge {
    pub fn new(
        edge_type: LineageEdgeType,
        source_node_id: LineageNodeId,
        target_node_id: LineageNodeId,
    ) -> Self {
        Self {
            id: LineageEdgeId::new(),
            edge_type,
            source_node_id,
            target_node_id,
            description: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLineage {
    pub nodes: HashMap<LineageNodeId, LineageNode>,
    pub edges: Vec<LineageEdge>,
}

impl DataLineage {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: LineageNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: LineageEdge) {
        self.edges.push(edge);
    }

    pub fn get_node(&self, node_id: &LineageNodeId) -> Option<&LineageNode> {
        self.nodes.get(node_id)
    }

    pub fn get_outgoing_edges(&self, node_id: &LineageNodeId) -> Vec<&LineageEdge> {
        self.edges
            .iter()
            .filter(|e| e.source_node_id == *node_id)
            .collect()
    }

    pub fn get_incoming_edges(&self, node_id: &LineageNodeId) -> Vec<&LineageEdge> {
        self.edges
            .iter()
            .filter(|e| e.target_node_id == *node_id)
            .collect()
    }
}

impl Default for DataLineage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
pub trait LineageTracker: Send + Sync {
    async fn record_node(&self, node: LineageNode) -> crate::utils::Result<LineageNodeId>;
    async fn record_edge(&self, edge: LineageEdge) -> crate::utils::Result<LineageEdgeId>;
    async fn get_node(&self, node_id: &LineageNodeId) -> crate::utils::Result<Option<LineageNode>>;
    async fn get_lineage(&self, node_id: &LineageNodeId) -> crate::utils::Result<DataLineage>;
    async fn query_upstream(
        &self,
        node_id: &LineageNodeId,
        depth: Option<usize>,
    ) -> crate::utils::Result<Vec<LineageNode>>;
    async fn query_downstream(
        &self,
        node_id: &LineageNodeId,
        depth: Option<usize>,
    ) -> crate::utils::Result<Vec<LineageNode>>;
}

pub use lineage::{
    LineageCollector, LineageGraphBuilder, LineageQueryEngine, LineageRecord, LineageStore,
    LineageVisualizationData, VisualizationFormat,
};

pub use classification::{
    ClassificationManager, ClassificationResult, ClassificationStrategy, ClassificationTag,
    ContentBasedClassifier, DataClassification, DataClassifier, MetadataBasedClassifier,
    ReviewStatus, ReviewTask,
};

pub use masking::{
    DataMasker, DynamicMasker, MaskingAlgorithm, MaskingConfig, MaskingException, MaskingManager,
    MaskingResult, MaskingRule, StaticMasker,
};

pub use compliance_reporting::{
    CheckStatus, ComplianceCheck, ComplianceReport, ComplianceReportGenerator, ComplianceStandard,
    ReportFormat, ReportTemplate, ReportType, ScheduledReport,
};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_lineage_node_id_new() {
        let id = LineageNodeId::new();
        assert!(!id.0.is_empty());
    }

    #[test]
    fn test_lineage_node_id_from_string() {
        let test_id = "test-id-123".to_string();
        let id = LineageNodeId::from_string(test_id.clone());
        assert_eq!(id.0, test_id);
    }

    #[test]
    fn test_lineage_node_id_default() {
        let id = LineageNodeId::default();
        assert!(!id.0.is_empty());
    }

    #[test]
    fn test_lineage_node_new() {
        let node = LineageNode::new(LineageNodeType::Table, "test_table".to_string());
        assert_eq!(node.name, "test_table");
        assert_eq!(node.node_type, LineageNodeType::Table);
        assert!(node.description.is_none());
        assert!(node.metadata.is_empty());
    }

    #[test]
    fn test_lineage_node_with_description() {
        let node = LineageNode::new(LineageNodeType::Table, "test_table".to_string())
            .with_description("Test description".to_string());
        assert_eq!(node.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_lineage_node_with_metadata() {
        let node = LineageNode::new(LineageNodeType::Table, "test_table".to_string())
            .with_metadata("key".to_string(), json!("value"));
        assert!(node.metadata.contains_key("key"));
    }

    #[test]
    fn test_lineage_edge_id_new() {
        let id = LineageEdgeId::new();
        assert!(!id.0.is_empty());
    }

    #[test]
    fn test_lineage_edge_id_from_string() {
        let test_id = "test-edge-id-123".to_string();
        let id = LineageEdgeId::from_string(test_id.clone());
        assert_eq!(id.0, test_id);
    }

    #[test]
    fn test_lineage_edge_id_default() {
        let id = LineageEdgeId::default();
        assert!(!id.0.is_empty());
    }

    #[test]
    fn test_lineage_edge_new() {
        let source_id = LineageNodeId::new();
        let target_id = LineageNodeId::new();

        let edge = LineageEdge::new(
            LineageEdgeType::ReadsFrom,
            source_id.clone(),
            target_id.clone(),
        );

        assert_eq!(edge.edge_type, LineageEdgeType::ReadsFrom);
        assert_eq!(edge.source_node_id, source_id);
        assert_eq!(edge.target_node_id, target_id);
    }

    #[test]
    fn test_lineage_edge_with_description() {
        let source_id = LineageNodeId::new();
        let target_id = LineageNodeId::new();

        let edge = LineageEdge::new(LineageEdgeType::ReadsFrom, source_id, target_id)
            .with_description("Edge description".to_string());

        assert_eq!(edge.description, Some("Edge description".to_string()));
    }

    #[test]
    fn test_lineage_edge_with_metadata() {
        let source_id = LineageNodeId::new();
        let target_id = LineageNodeId::new();

        let edge = LineageEdge::new(LineageEdgeType::ReadsFrom, source_id, target_id)
            .with_metadata("key".to_string(), json!("value"));

        assert!(edge.metadata.contains_key("key"));
    }

    #[test]
    fn test_data_lineage_new() {
        let lineage = DataLineage::new();
        assert!(lineage.nodes.is_empty());
        assert!(lineage.edges.is_empty());
    }

    #[test]
    fn test_data_lineage_default() {
        let lineage = DataLineage::default();
        assert!(lineage.nodes.is_empty());
        assert!(lineage.edges.is_empty());
    }

    #[test]
    fn test_data_lineage_add_node() {
        let mut lineage = DataLineage::new();
        let node = LineageNode::new(LineageNodeType::Table, "test".to_string());
        let node_id = node.id.clone();

        lineage.add_node(node);
        assert!(lineage.nodes.contains_key(&node_id));
    }

    #[test]
    fn test_data_lineage_add_edge() {
        let mut lineage = DataLineage::new();
        let source_id = LineageNodeId::new();
        let target_id = LineageNodeId::new();
        let edge = LineageEdge::new(LineageEdgeType::ReadsFrom, source_id, target_id);

        lineage.add_edge(edge);
        assert_eq!(lineage.edges.len(), 1);
    }

    #[test]
    fn test_data_lineage_get_node() {
        let mut lineage = DataLineage::new();
        let node = LineageNode::new(LineageNodeType::Table, "test".to_string());
        let node_id = node.id.clone();

        lineage.add_node(node);
        let retrieved = lineage.get_node(&node_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test");
    }

    #[test]
    fn test_data_lineage_get_outgoing_edges() {
        let mut lineage = DataLineage::new();
        let source_id = LineageNodeId::new();
        let target_id1 = LineageNodeId::new();
        let target_id2 = LineageNodeId::new();

        let edge1 = LineageEdge::new(LineageEdgeType::ReadsFrom, source_id.clone(), target_id1);
        let edge2 = LineageEdge::new(LineageEdgeType::WritesTo, source_id.clone(), target_id2);

        lineage.add_edge(edge1);
        lineage.add_edge(edge2);

        let outgoing = lineage.get_outgoing_edges(&source_id);
        assert_eq!(outgoing.len(), 2);
    }

    #[test]
    fn test_data_lineage_get_incoming_edges() {
        let mut lineage = DataLineage::new();
        let source_id1 = LineageNodeId::new();
        let source_id2 = LineageNodeId::new();
        let target_id = LineageNodeId::new();

        let edge1 = LineageEdge::new(LineageEdgeType::ReadsFrom, source_id1, target_id.clone());
        let edge2 = LineageEdge::new(LineageEdgeType::WritesTo, source_id2, target_id.clone());

        lineage.add_edge(edge1);
        lineage.add_edge(edge2);

        let incoming = lineage.get_incoming_edges(&target_id);
        assert_eq!(incoming.len(), 2);
    }

    #[test]
    fn test_lineage_node_type_equality() {
        assert_eq!(LineageNodeType::Table, LineageNodeType::Table);
        assert_eq!(LineageNodeType::Column, LineageNodeType::Column);
        assert_eq!(LineageNodeType::DataSource, LineageNodeType::DataSource);
    }

    #[test]
    fn test_lineage_edge_type_equality() {
        assert_eq!(LineageEdgeType::ReadsFrom, LineageEdgeType::ReadsFrom);
        assert_eq!(LineageEdgeType::WritesTo, LineageEdgeType::WritesTo);
        assert_eq!(LineageEdgeType::TransformsTo, LineageEdgeType::TransformsTo);
    }
}
