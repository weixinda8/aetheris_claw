use crate::data_governance::{
    DataLineage, LineageEdgeType, LineageNodeType,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub description: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub edge_type: String,
    pub description: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageVisualizationData {
    pub nodes: Vec<VisualizationNode>,
    pub edges: Vec<VisualizationEdge>,
    pub format: VisualizationFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VisualizationFormat {
    D3Js,
    CytoscapeJs,
    Generic,
}

impl LineageVisualizationData {
    pub fn from_lineage(lineage: &DataLineage, format: VisualizationFormat) -> Self {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for (node_id, node) in &lineage.nodes {
            nodes.push(VisualizationNode {
                id: node_id.0.clone(),
                label: node.name.clone(),
                node_type: format_node_type(&node.node_type),
                description: node.description.clone(),
                metadata: serde_json::json!(node.metadata),
                created_at: node.created_at.to_rfc3339(),
            });
        }

        for edge in &lineage.edges {
            edges.push(VisualizationEdge {
                id: edge.id.0.clone(),
                source: edge.source_node_id.0.clone(),
                target: edge.target_node_id.0.clone(),
                edge_type: format_edge_type(&edge.edge_type),
                description: edge.description.clone(),
                metadata: serde_json::json!(edge.metadata),
            });
        }

        Self {
            nodes,
            edges,
            format,
        }
    }

    pub fn to_d3_js(&self) -> serde_json::Value {
        serde_json::json!({
            "nodes": self.nodes.iter().map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "name": n.label,
                    "type": n.node_type,
                    "description": n.description,
                    "metadata": n.metadata,
                })
            }).collect::<Vec<_>>(),
            "links": self.edges.iter().map(|e| {
                serde_json::json!({
                    "source": e.source,
                    "target": e.target,
                    "type": e.edge_type,
                    "description": e.description,
                })
            }).collect::<Vec<_>>(),
        })
    }

    pub fn to_cytoscape_js(&self) -> serde_json::Value {
        let mut elements = Vec::new();

        for node in &self.nodes {
            elements.push(serde_json::json!({
                "data": {
                    "id": node.id,
                    "label": node.label,
                    "type": node.node_type,
                    "description": node.description,
                    "metadata": node.metadata,
                }
            }));
        }

        for edge in &self.edges {
            elements.push(serde_json::json!({
                "data": {
                    "id": edge.id,
                    "source": edge.source,
                    "target": edge.target,
                    "type": edge.edge_type,
                    "description": edge.description,
                    "metadata": edge.metadata,
                }
            }));
        }

        serde_json::json!({
            "elements": elements,
            "style": [
                {
                    "selector": "node",
                    "style": {
                        "label": "data(label)",
                        "background-color": "#666",
                        "width": "60px",
                        "height": "60px",
                    }
                },
                {
                    "selector": "edge",
                    "style": {
                        "label": "data(type)",
                        "line-color": "#999",
                        "target-arrow-color": "#999",
                        "target-arrow-shape": "triangle",
                        "curve-style": "bezier",
                    }
                },
            ],
            "layout": {
                "name": "cose",
            },
        })
    }

    pub fn to_generic(&self) -> serde_json::Value {
        serde_json::json!({
            "nodes": self.nodes,
            "edges": self.edges,
        })
    }

    pub fn export(&self) -> serde_json::Value {
        match self.format {
            VisualizationFormat::D3Js => self.to_d3_js(),
            VisualizationFormat::CytoscapeJs => self.to_cytoscape_js(),
            VisualizationFormat::Generic => self.to_generic(),
        }
    }
}

fn format_node_type(node_type: &LineageNodeType) -> String {
    match node_type {
        LineageNodeType::DataSource => "data_source".to_string(),
        LineageNodeType::DataTransform => "data_transform".to_string(),
        LineageNodeType::DataSink => "data_sink".to_string(),
        LineageNodeType::Table => "table".to_string(),
        LineageNodeType::Column => "column".to_string(),
        LineageNodeType::File => "file".to_string(),
        LineageNodeType::Api => "api".to_string(),
        LineageNodeType::Agent => "agent".to_string(),
        LineageNodeType::Task => "task".to_string(),
        LineageNodeType::Workflow => "workflow".to_string(),
        LineageNodeType::Custom(s) => format!("custom_{}", s),
    }
}

fn format_edge_type(edge_type: &LineageEdgeType) -> String {
    match edge_type {
        LineageEdgeType::ReadsFrom => "reads_from".to_string(),
        LineageEdgeType::WritesTo => "writes_to".to_string(),
        LineageEdgeType::TransformsTo => "transforms_to".to_string(),
        LineageEdgeType::References => "references".to_string(),
        LineageEdgeType::DependsOn => "depends_on".to_string(),
        LineageEdgeType::Custom(s) => format!("custom_{}", s),
    }
}
