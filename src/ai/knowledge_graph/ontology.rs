use super::*;

pub struct IndustrialOntology {
    ontology: Ontology,
}

impl IndustrialOntology {
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        let mut entity_types = HashMap::new();
        let mut relationship_types = HashMap::new();

        entity_types.insert(
            "Device".to_string(),
            EntityTypeDefinition {
                name: "Device".to_string(),
                description: Some("工业设备或机器".to_string()),
                parent_types: Vec::new(),
                required_properties: vec!["device_type".to_string()],
                optional_properties: vec![
                    "model".to_string(),
                    "serial_number".to_string(),
                    "manufacturer".to_string(),
                    "installation_date".to_string(),
                ],
            },
        );

        entity_types.insert(
            "Fault".to_string(),
            EntityTypeDefinition {
                name: "Fault".to_string(),
                description: Some("设备故障或异常".to_string()),
                parent_types: Vec::new(),
                required_properties: Vec::new(),
                optional_properties: vec![
                    "fault_code".to_string(),
                    "severity".to_string(),
                    "occurrence_time".to_string(),
                ],
            },
        );

        entity_types.insert(
            "Solution".to_string(),
            EntityTypeDefinition {
                name: "Solution".to_string(),
                description: Some("故障解决方案".to_string()),
                parent_types: Vec::new(),
                required_properties: Vec::new(),
                optional_properties: vec![
                    "steps".to_string(),
                    "estimated_time".to_string(),
                    "required_tools".to_string(),
                ],
            },
        );

        entity_types.insert(
            "MaintenanceCase".to_string(),
            EntityTypeDefinition {
                name: "MaintenanceCase".to_string(),
                description: Some("维护案例记录".to_string()),
                parent_types: Vec::new(),
                required_properties: Vec::new(),
                optional_properties: vec![
                    "resolution_summary".to_string(),
                    "root_cause".to_string(),
                    "duration".to_string(),
                ],
            },
        );

        entity_types.insert(
            "Component".to_string(),
            EntityTypeDefinition {
                name: "Component".to_string(),
                description: Some("设备组件".to_string()),
                parent_types: Vec::new(),
                required_properties: Vec::new(),
                optional_properties: vec!["part_number".to_string(), "specification".to_string()],
            },
        );

        relationship_types.insert(
            "HasFault".to_string(),
            RelationshipTypeDefinition {
                name: "HasFault".to_string(),
                description: Some("设备发生故障".to_string()),
                source_entity_types: vec![EntityType::Device],
                target_entity_types: vec![EntityType::Fault],
                required_properties: Vec::new(),
                optional_properties: vec!["occurrence_date".to_string()],
            },
        );

        relationship_types.insert(
            "CausedByDevice".to_string(),
            RelationshipTypeDefinition {
                name: "CausedByDevice".to_string(),
                description: Some("故障由设备导致".to_string()),
                source_entity_types: vec![EntityType::Fault],
                target_entity_types: vec![EntityType::Device],
                required_properties: Vec::new(),
                optional_properties: Vec::new(),
            },
        );

        relationship_types.insert(
            "HasSolution".to_string(),
            RelationshipTypeDefinition {
                name: "HasSolution".to_string(),
                description: Some("故障有解决方案".to_string()),
                source_entity_types: vec![EntityType::Fault],
                target_entity_types: vec![EntityType::Solution],
                required_properties: Vec::new(),
                optional_properties: vec!["success_rate".to_string()],
            },
        );

        relationship_types.insert(
            "SolvesFault".to_string(),
            RelationshipTypeDefinition {
                name: "SolvesFault".to_string(),
                description: Some("解决方案解决故障".to_string()),
                source_entity_types: vec![EntityType::Solution],
                target_entity_types: vec![EntityType::Fault],
                required_properties: Vec::new(),
                optional_properties: Vec::new(),
            },
        );

        relationship_types.insert(
            "UsesComponent".to_string(),
            RelationshipTypeDefinition {
                name: "UsesComponent".to_string(),
                description: Some("设备使用组件".to_string()),
                source_entity_types: vec![EntityType::Device],
                target_entity_types: vec![EntityType::Component],
                required_properties: Vec::new(),
                optional_properties: vec!["quantity".to_string()],
            },
        );

        relationship_types.insert(
            "PartOf".to_string(),
            RelationshipTypeDefinition {
                name: "PartOf".to_string(),
                description: Some("组件是设备的一部分".to_string()),
                source_entity_types: vec![EntityType::Component],
                target_entity_types: vec![EntityType::Device],
                required_properties: Vec::new(),
                optional_properties: Vec::new(),
            },
        );

        Self {
            ontology: Ontology {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Industrial Maintenance Ontology".to_string(),
                description: Some("工业维护领域本体".to_string()),
                entity_types,
                relationship_types,
                created_at: now,
            },
        }
    }

    pub fn ontology(&self) -> &Ontology {
        &self.ontology
    }

    pub fn get_entity_type_definition(
        &self,
        entity_type: &EntityType,
    ) -> Option<&EntityTypeDefinition> {
        let type_name = match entity_type {
            EntityType::Device => "Device",
            EntityType::Fault => "Fault",
            EntityType::Solution => "Solution",
            EntityType::MaintenanceCase => "MaintenanceCase",
            EntityType::Component => "Component",
            EntityType::Process => "Process",
            EntityType::Material => "Material",
            EntityType::Operator => "Operator",
            EntityType::Custom(name) => name,
        };
        self.ontology.entity_types.get(type_name)
    }

    pub fn get_relationship_type_definition(
        &self,
        relationship_type: &RelationshipType,
    ) -> Option<&RelationshipTypeDefinition> {
        let type_name = match relationship_type {
            RelationshipType::HasFault => "HasFault",
            RelationshipType::CausedByDevice => "CausedByDevice",
            RelationshipType::HasSolution => "HasSolution",
            RelationshipType::SolvesFault => "SolvesFault",
            RelationshipType::UsesComponent => "UsesComponent",
            RelationshipType::PartOf => "PartOf",
            RelationshipType::Requires => "Requires",
            RelationshipType::SimilarTo => "SimilarTo",
            RelationshipType::CausedBy => "CausedBy",
            RelationshipType::PreventedBy => "PreventedBy",
            RelationshipType::Custom(name) => name,
        };
        self.ontology.relationship_types.get(type_name)
    }

    pub fn validate_entity(&self, entity: &Entity) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if let Some(def) = self.get_entity_type_definition(&entity.entity_type) {
            for required_prop in &def.required_properties {
                if !entity.properties.contains_key(required_prop) {
                    errors.push(format!("缺少必需属性: {}", required_prop));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn validate_relationship(
        &self,
        relationship: &Relationship,
        source_entity: &Entity,
        target_entity: &Entity,
    ) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if let Some(def) = self.get_relationship_type_definition(&relationship.relationship_type) {
            if !def.source_entity_types.contains(&source_entity.entity_type) {
                errors.push(format!(
                    "源实体类型 {:?} 不匹配关系要求 {:?}",
                    source_entity.entity_type, def.source_entity_types
                ));
            }

            if !def.target_entity_types.contains(&target_entity.entity_type) {
                errors.push(format!(
                    "目标实体类型 {:?} 不匹配关系要求 {:?}",
                    target_entity.entity_type, def.target_entity_types
                ));
            }

            for required_prop in &def.required_properties {
                if !relationship.properties.contains_key(required_prop) {
                    errors.push(format!("缺少必需属性: {}", required_prop));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for IndustrialOntology {
    fn default() -> Self {
        Self::new()
    }
}
