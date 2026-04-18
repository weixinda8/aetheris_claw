use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub entity_type: EntityType,
    pub description: Option<String>,
    pub properties: HashMap<String, serde_json::Value>,
    pub group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relationship_type: RelationshipType,
    pub label: String,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphVisualizationData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl GraphVisualizationData {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn from_graph(graph: &dyn KnowledgeGraph) -> Self {
        let mut data = Self::new();

        for entity in graph.list_entities(None) {
            data.add_entity(entity);
        }

        for relationship in graph.list_relationships(None) {
            data.add_relationship(relationship);
        }

        data.metadata.insert(
            "node_count".to_string(),
            serde_json::json!(data.nodes.len()),
        );
        data.metadata.insert(
            "edge_count".to_string(),
            serde_json::json!(data.edges.len()),
        );

        data
    }

    pub fn add_entity(&mut self, entity: Entity) {
        let group = match entity.entity_type {
            EntityType::Device => "device".to_string(),
            EntityType::Fault => "fault".to_string(),
            EntityType::Solution => "solution".to_string(),
            EntityType::MaintenanceCase => "case".to_string(),
            EntityType::Component => "component".to_string(),
            EntityType::Process => "process".to_string(),
            EntityType::Material => "material".to_string(),
            EntityType::Operator => "operator".to_string(),
            EntityType::Custom(_) => "custom".to_string(),
        };

        let node = GraphNode {
            id: entity.id,
            label: entity.name,
            entity_type: entity.entity_type,
            description: entity.description,
            properties: entity.properties,
            group,
        };

        self.nodes.push(node);
    }

    pub fn add_relationship(&mut self, relationship: Relationship) {
        let label = match relationship.relationship_type {
            RelationshipType::HasFault => "发生故障".to_string(),
            RelationshipType::CausedByDevice => "由设备导致".to_string(),
            RelationshipType::HasSolution => "有解决方案".to_string(),
            RelationshipType::SolvesFault => "解决故障".to_string(),
            RelationshipType::UsesComponent => "使用组件".to_string(),
            RelationshipType::PartOf => "属于".to_string(),
            RelationshipType::Requires => "需要".to_string(),
            RelationshipType::SimilarTo => "相似于".to_string(),
            RelationshipType::CausedBy => "由...导致".to_string(),
            RelationshipType::PreventedBy => "被...预防".to_string(),
            RelationshipType::Custom(ref name) => name.clone(),
        };

        let edge = GraphEdge {
            id: relationship.id,
            source: relationship.source_id,
            target: relationship.target_id,
            relationship_type: relationship.relationship_type,
            label,
            properties: relationship.properties,
        };

        self.edges.push(edge);
    }

    pub fn filter_by_entity_type(mut self, entity_types: &[EntityType]) -> Self {
        let node_ids: std::collections::HashSet<_> = self
            .nodes
            .iter()
            .filter(|node| entity_types.contains(&node.entity_type))
            .map(|node| node.id.clone())
            .collect();

        self.nodes.retain(|node| node_ids.contains(&node.id));
        self.edges
            .retain(|edge| node_ids.contains(&edge.source) && node_ids.contains(&edge.target));

        self.metadata.insert(
            "node_count".to_string(),
            serde_json::json!(self.nodes.len()),
        );
        self.metadata.insert(
            "edge_count".to_string(),
            serde_json::json!(self.edges.len()),
        );

        self
    }

    pub fn filter_by_relationship_type(mut self, relationship_types: &[RelationshipType]) -> Self {
        self.edges
            .retain(|edge| relationship_types.contains(&edge.relationship_type));

        let node_ids: std::collections::HashSet<_> = self
            .edges
            .iter()
            .flat_map(|edge| vec![edge.source.clone(), edge.target.clone()])
            .collect();

        self.nodes.retain(|node| node_ids.contains(&node.id));

        self.metadata.insert(
            "node_count".to_string(),
            serde_json::json!(self.nodes.len()),
        );
        self.metadata.insert(
            "edge_count".to_string(),
            serde_json::json!(self.edges.len()),
        );

        self
    }

    pub fn limit_nodes(mut self, max_nodes: usize) -> Self {
        if self.nodes.len() > max_nodes {
            self.nodes.truncate(max_nodes);

            let node_ids: std::collections::HashSet<_> =
                self.nodes.iter().map(|node| node.id.clone()).collect();

            self.edges
                .retain(|edge| node_ids.contains(&edge.source) && node_ids.contains(&edge.target));

            self.metadata.insert(
                "node_count".to_string(),
                serde_json::json!(self.nodes.len()),
            );
            self.metadata.insert(
                "edge_count".to_string(),
                serde_json::json!(self.edges.len()),
            );
            self.metadata
                .insert("truncated".to_string(), serde_json::json!(true));
        }

        self
    }

    pub fn to_cytoscape_format(&self) -> serde_json::Value {
        let elements: Vec<serde_json::Value> = self
            .nodes
            .iter()
            .map(|node| {
                serde_json::json!({
                    "data": {
                        "id": node.id,
                        "label": node.label,
                        "type": format!("{:?}", node.entity_type),
                        "group": node.group,
                        "description": node.description,
                        "properties": node.properties,
                    }
                })
            })
            .chain(self.edges.iter().map(|edge| {
                serde_json::json!({
                    "data": {
                        "id": edge.id,
                        "source": edge.source,
                        "target": edge.target,
                        "label": edge.label,
                        "type": format!("{:?}", edge.relationship_type),
                        "properties": edge.properties,
                    }
                })
            }))
            .collect();

        serde_json::json!({
            "elements": elements,
            "metadata": self.metadata,
        })
    }

    pub fn to_d3_format(&self) -> serde_json::Value {
        serde_json::json!({
            "nodes": self.nodes,
            "links": self.edges.iter().map(|edge| {
                serde_json::json!({
                    "source": edge.source,
                    "target": edge.target,
                    "id": edge.id,
                    "type": format!("{:?}", edge.relationship_type),
                    "label": edge.label,
                    "properties": edge.properties,
                })
            }).collect::<Vec<_>>(),
            "metadata": self.metadata,
        })
    }
}

impl Default for GraphVisualizationData {
    fn default() -> Self {
        Self::new()
    }
}
