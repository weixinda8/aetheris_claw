use super::*;

pub struct IndustrialKnowledgeGraph {
    graph: InMemoryKnowledgeGraph,
}

impl IndustrialKnowledgeGraph {
    pub fn new() -> Self {
        Self {
            graph: InMemoryKnowledgeGraph::new(),
        }
    }

    pub fn add_device(
        &mut self,
        name: String,
        description: Option<String>,
        device_type: String,
        model: Option<String>,
        serial_number: Option<String>,
    ) -> crate::utils::Result<String> {
        let mut entity = Entity::new(EntityType::Device, name, description);
        entity = entity.with_property("device_type".to_string(), serde_json::json!(device_type));
        if let Some(m) = model {
            entity = entity.with_property("model".to_string(), serde_json::json!(m));
        }
        if let Some(sn) = serial_number {
            entity = entity.with_property("serial_number".to_string(), serde_json::json!(sn));
        }
        self.graph.add_entity(entity)
    }

    pub fn add_fault(
        &mut self,
        name: String,
        description: Option<String>,
        fault_code: Option<String>,
        severity: Option<String>,
    ) -> crate::utils::Result<String> {
        let mut entity = Entity::new(EntityType::Fault, name, description);
        if let Some(fc) = fault_code {
            entity = entity.with_property("fault_code".to_string(), serde_json::json!(fc));
        }
        if let Some(s) = severity {
            entity = entity.with_property("severity".to_string(), serde_json::json!(s));
        }
        self.graph.add_entity(entity)
    }

    pub fn add_solution(
        &mut self,
        name: String,
        description: Option<String>,
        steps: Option<Vec<String>>,
        estimated_time: Option<String>,
    ) -> crate::utils::Result<String> {
        let mut entity = Entity::new(EntityType::Solution, name, description);
        if let Some(s) = steps {
            entity = entity.with_property("steps".to_string(), serde_json::json!(s));
        }
        if let Some(et) = estimated_time {
            entity = entity.with_property("estimated_time".to_string(), serde_json::json!(et));
        }
        self.graph.add_entity(entity)
    }

    pub fn link_device_to_fault(
        &mut self,
        device_id: String,
        fault_id: String,
    ) -> crate::utils::Result<String> {
        let rel = Relationship::new(
            RelationshipType::HasFault,
            device_id.clone(),
            fault_id.clone(),
        );
        self.graph.add_relationship(rel)?;

        let rel_reverse = Relationship::new(RelationshipType::CausedByDevice, fault_id, device_id);
        self.graph.add_relationship(rel_reverse)
    }

    pub fn link_fault_to_solution(
        &mut self,
        fault_id: String,
        solution_id: String,
    ) -> crate::utils::Result<String> {
        let rel = Relationship::new(
            RelationshipType::HasSolution,
            fault_id.clone(),
            solution_id.clone(),
        );
        self.graph.add_relationship(rel)?;

        let rel_reverse = Relationship::new(RelationshipType::SolvesFault, solution_id, fault_id);
        self.graph.add_relationship(rel_reverse)
    }

    pub fn get_device_faults(&self, device_id: &str) -> Vec<Entity> {
        self.graph
            .get_relationships_from(device_id)
            .iter()
            .filter(|rel| rel.relationship_type == RelationshipType::HasFault)
            .filter_map(|rel| self.graph.get_entity(&rel.target_id))
            .collect()
    }

    pub fn get_fault_solutions(&self, fault_id: &str) -> Vec<Entity> {
        self.graph
            .get_relationships_from(fault_id)
            .iter()
            .filter(|rel| rel.relationship_type == RelationshipType::HasSolution)
            .filter_map(|rel| self.graph.get_entity(&rel.target_id))
            .collect()
    }

    pub fn get_fault_causing_devices(&self, fault_id: &str) -> Vec<Entity> {
        self.graph
            .get_relationships_from(fault_id)
            .iter()
            .filter(|rel| rel.relationship_type == RelationshipType::CausedByDevice)
            .filter_map(|rel| self.graph.get_entity(&rel.target_id))
            .collect()
    }

    pub fn graph(&self) -> &InMemoryKnowledgeGraph {
        &self.graph
    }

    pub fn graph_mut(&mut self) -> &mut InMemoryKnowledgeGraph {
        &mut self.graph
    }
}

impl Default for IndustrialKnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}
