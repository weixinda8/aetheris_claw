use crate::digital_twin::{TwinEntity, TwinEntityType, TwinState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinVisualizationData {
    pub timestamp: DateTime<Utc>,
    pub entities: Vec<VisualizationEntity>,
    pub connections: Vec<VisualizationConnection>,
    pub bounds: VisualizationBounds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationEntity {
    pub id: String,
    pub name: String,
    pub entity_type: TwinEntityType,
    pub state: TwinState,
    pub position: Position3D,
    pub rotation: Rotation3D,
    pub scale: f64,
    pub properties: HashMap<String, serde_json::Value>,
    pub parent_id: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Position3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Rotation3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationConnection {
    pub id: String,
    pub from_entity_id: String,
    pub to_entity_id: String,
    pub connection_type: ConnectionType,
    pub color: Option<String>,
    pub thickness: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectionType {
    Physical,
    DataFlow,
    Control,
    Hierarchy,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationBounds {
    pub min: Position3D,
    pub max: Position3D,
}

impl TwinVisualizationData {
    pub fn from_entities(entities: Vec<TwinEntity>) -> Self {
        let mut viz_entities = Vec::new();
        let mut connections = Vec::new();

        let mut x = 0.0;
        let mut y = 0.0;
        let z = 0.0;

        for entity in entities {
            let viz_entity = VisualizationEntity {
                id: entity.id.clone(),
                name: entity.name.clone(),
                entity_type: entity.entity_type.clone(),
                state: entity.state.clone(),
                position: Position3D { x, y, z },
                rotation: Rotation3D::default(),
                scale: 1.0,
                properties: entity.properties.clone(),
                parent_id: entity.parent_id.clone(),
                color: Some(Self::state_to_color(&entity.state)),
            };

            viz_entities.push(viz_entity);

            if let Some(parent_id) = &entity.parent_id {
                connections.push(VisualizationConnection {
                    id: format!("conn-{}-{}", parent_id, entity.id),
                    from_entity_id: parent_id.clone(),
                    to_entity_id: entity.id.clone(),
                    connection_type: ConnectionType::Hierarchy,
                    color: Some("#666666".to_string()),
                    thickness: 1.0,
                });
            }

            x += 2.0;
            if x > 10.0 {
                x = 0.0;
                y += 2.0;
            }
        }

        let bounds = Self::calculate_bounds(&viz_entities);

        Self {
            timestamp: Utc::now(),
            entities: viz_entities,
            connections,
            bounds,
        }
    }

    fn state_to_color(state: &TwinState) -> String {
        match state {
            TwinState::Unknown => "#808080".to_string(),
            TwinState::Offline => "#444444".to_string(),
            TwinState::Online => "#00FF00".to_string(),
            TwinState::Degraded => "#FFFF00".to_string(),
            TwinState::Failed => "#FF0000".to_string(),
            TwinState::Maintenance => "#0088FF".to_string(),
        }
    }

    fn calculate_bounds(entities: &[VisualizationEntity]) -> VisualizationBounds {
        let mut min = Position3D {
            x: f64::INFINITY,
            y: f64::INFINITY,
            z: f64::INFINITY,
        };
        let mut max = Position3D {
            x: f64::NEG_INFINITY,
            y: f64::NEG_INFINITY,
            z: f64::NEG_INFINITY,
        };

        for entity in entities {
            min.x = min.x.min(entity.position.x);
            min.y = min.y.min(entity.position.y);
            min.z = min.z.min(entity.position.z);

            max.x = max.x.max(entity.position.x);
            max.y = max.y.max(entity.position.y);
            max.z = max.z.max(entity.position.z);
        }

        if min.x == f64::INFINITY {
            min = Position3D {
                x: -5.0,
                y: -5.0,
                z: -5.0,
            };
            max = Position3D {
                x: 5.0,
                y: 5.0,
                z: 5.0,
            };
        }

        VisualizationBounds { min, max }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::digital_twin::{TwinEntity, TwinEntityType, TwinState};
    use chrono::Utc;
    use std::collections::HashMap;

    #[test]
    fn test_position3d_default() {
        let pos = Position3D::default();
        assert_eq!(pos.x, 0.0);
        assert_eq!(pos.y, 0.0);
        assert_eq!(pos.z, 0.0);
    }

    #[test]
    fn test_rotation3d_default() {
        let rot = Rotation3D::default();
        assert_eq!(rot.x, 0.0);
        assert_eq!(rot.y, 0.0);
        assert_eq!(rot.z, 0.0);
        assert_eq!(rot.w, 0.0);
    }

    #[test]
    fn test_from_entities_empty() {
        let data = TwinVisualizationData::from_entities(Vec::new());
        assert!(data.entities.is_empty());
        assert!(data.connections.is_empty());
    }

    #[test]
    fn test_from_entities_single() {
        let entity = TwinEntity {
            id: "test-1".to_string(),
            name: "Test Entity".to_string(),
            entity_type: TwinEntityType::Device,
            properties: HashMap::new(),
            state: TwinState::Online,
            parent_id: None,
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let data = TwinVisualizationData::from_entities(vec![entity]);
        assert_eq!(data.entities.len(), 1);
        assert_eq!(data.connections.len(), 0);
        assert_eq!(data.entities[0].name, "Test Entity");
        assert_eq!(data.entities[0].state, TwinState::Online);
    }

    #[test]
    fn test_from_entities_with_parent() {
        let parent = TwinEntity {
            id: "parent-1".to_string(),
            name: "Parent".to_string(),
            entity_type: TwinEntityType::Line,
            properties: HashMap::new(),
            state: TwinState::Online,
            parent_id: None,
            children_ids: vec!["child-1".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let child = TwinEntity {
            id: "child-1".to_string(),
            name: "Child".to_string(),
            entity_type: TwinEntityType::Device,
            properties: HashMap::new(),
            state: TwinState::Online,
            parent_id: Some("parent-1".to_string()),
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let data = TwinVisualizationData::from_entities(vec![parent, child]);
        assert_eq!(data.entities.len(), 2);
        assert_eq!(data.connections.len(), 1);
        assert_eq!(
            data.connections[0].connection_type,
            ConnectionType::Hierarchy
        );
    }

    #[test]
    fn test_state_colors() {
        let create_entity_with_state = |state: TwinState| -> TwinEntity {
            TwinEntity {
                id: "test".to_string(),
                name: "Test".to_string(),
                entity_type: TwinEntityType::Device,
                properties: HashMap::new(),
                state,
                parent_id: None,
                children_ids: Vec::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        };

        let data = TwinVisualizationData::from_entities(vec![
            create_entity_with_state(TwinState::Online),
            create_entity_with_state(TwinState::Failed),
            create_entity_with_state(TwinState::Degraded),
        ]);

        assert_eq!(data.entities.len(), 3);
        assert_eq!(data.entities[0].color, Some("#00FF00".to_string()));
        assert_eq!(data.entities[1].color, Some("#FF0000".to_string()));
        assert_eq!(data.entities[2].color, Some("#FFFF00".to_string()));
    }
}
