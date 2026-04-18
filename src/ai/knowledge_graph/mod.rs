pub mod cases;
pub mod industrial;
pub mod ontology;
pub mod search;
pub mod storage;
pub mod visualization;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EntityType {
    Device,
    Fault,
    Solution,
    MaintenanceCase,
    Component,
    Process,
    Material,
    Operator,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub entity_type: EntityType,
    pub name: String,
    pub description: Option<String>,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Entity {
    pub fn new(entity_type: EntityType, name: String, description: Option<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            entity_type,
            name,
            description,
            properties: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_property(mut self, key: String, value: serde_json::Value) -> Self {
        self.properties.insert(key, value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RelationshipType {
    HasFault,
    CausedByDevice,
    HasSolution,
    SolvesFault,
    UsesComponent,
    PartOf,
    Requires,
    SimilarTo,
    CausedBy,
    PreventedBy,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub relationship_type: RelationshipType,
    pub source_id: String,
    pub target_id: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Relationship {
    pub fn new(relationship_type: RelationshipType, source_id: String, target_id: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            relationship_type,
            source_id,
            target_id,
            properties: HashMap::new(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn with_property(mut self, key: String, value: serde_json::Value) -> Self {
        self.properties.insert(key, value);
        self
    }
}

pub trait KnowledgeGraph: Send + Sync {
    fn add_entity(&mut self, entity: Entity) -> crate::utils::Result<String>;
    fn get_entity(&self, entity_id: &str) -> Option<Entity>;
    fn update_entity(&mut self, entity: Entity) -> crate::utils::Result<()>;
    fn delete_entity(&mut self, entity_id: &str) -> crate::utils::Result<bool>;
    fn list_entities(&self, entity_type: Option<EntityType>) -> Vec<Entity>;

    fn add_relationship(&mut self, relationship: Relationship) -> crate::utils::Result<String>;
    fn get_relationship(&self, relationship_id: &str) -> Option<Relationship>;
    fn delete_relationship(&mut self, relationship_id: &str) -> crate::utils::Result<bool>;
    fn list_relationships(&self, relationship_type: Option<RelationshipType>)
    -> Vec<Relationship>;

    fn get_relationships_from(&self, entity_id: &str) -> Vec<Relationship>;
    fn get_relationships_to(&self, entity_id: &str) -> Vec<Relationship>;

    fn find_entities_by_name(&self, name: &str) -> Vec<Entity>;
    fn find_entities_by_property(&self, key: &str, value: &serde_json::Value) -> Vec<Entity>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ontology {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub entity_types: HashMap<String, EntityTypeDefinition>,
    pub relationship_types: HashMap<String, RelationshipTypeDefinition>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTypeDefinition {
    pub name: String,
    pub description: Option<String>,
    pub parent_types: Vec<String>,
    pub required_properties: Vec<String>,
    pub optional_properties: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipTypeDefinition {
    pub name: String,
    pub description: Option<String>,
    pub source_entity_types: Vec<EntityType>,
    pub target_entity_types: Vec<EntityType>,
    pub required_properties: Vec<String>,
    pub optional_properties: Vec<String>,
}

pub use cases::{CaseLibrary, MaintenanceCase};
pub use industrial::IndustrialKnowledgeGraph;
pub use ontology::IndustrialOntology;
pub use search::SemanticSearchEngine;
pub use storage::InMemoryKnowledgeGraph;
pub use visualization::GraphVisualizationData;
