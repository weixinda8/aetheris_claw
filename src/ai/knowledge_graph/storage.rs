use super::*;
use dashmap::DashMap;

pub struct InMemoryKnowledgeGraph {
    entities: DashMap<String, Entity>,
    relationships: DashMap<String, Relationship>,
    relationships_from: DashMap<String, Vec<String>>,
    relationships_to: DashMap<String, Vec<String>>,
}

impl InMemoryKnowledgeGraph {
    pub fn new() -> Self {
        Self {
            entities: DashMap::new(),
            relationships: DashMap::new(),
            relationships_from: DashMap::new(),
            relationships_to: DashMap::new(),
        }
    }
}

impl Default for InMemoryKnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeGraph for InMemoryKnowledgeGraph {
    fn add_entity(&mut self, entity: Entity) -> crate::utils::Result<String> {
        let entity_id = entity.id.clone();
        self.entities.insert(entity_id.clone(), entity);
        Ok(entity_id)
    }

    fn get_entity(&self, entity_id: &str) -> Option<Entity> {
        self.entities.get(entity_id).map(|entry| entry.value().clone())
    }

    fn update_entity(&mut self, mut entity: Entity) -> crate::utils::Result<()> {
        entity.updated_at = chrono::Utc::now();
        self.entities.insert(entity.id.clone(), entity);
        Ok(())
    }

    fn delete_entity(&mut self, entity_id: &str) -> crate::utils::Result<bool> {
        if let Some((_, _)) = self.entities.remove(entity_id) {
            if let Some((_, rel_ids)) = self.relationships_from.remove(entity_id) {
                for rel_id in rel_ids {
                    self.relationships.remove(&rel_id);
                }
            }
            if let Some((_, rel_ids)) = self.relationships_to.remove(entity_id) {
                for rel_id in rel_ids {
                    self.relationships.remove(&rel_id);
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn list_entities(&self, entity_type: Option<EntityType>) -> Vec<Entity> {
        self.entities
            .iter()
            .map(|entry| entry.value().clone())
            .filter(|entity| {
                if let Some(ref et) = entity_type {
                    &entity.entity_type == et
                } else {
                    true
                }
            })
            .collect()
    }

    fn add_relationship(&mut self, relationship: Relationship) -> crate::utils::Result<String> {
        let rel_id = relationship.id.clone();
        let source_id = relationship.source_id.clone();
        let target_id = relationship.target_id.clone();

        self.relationships.insert(rel_id.clone(), relationship);

        self.relationships_from
            .entry(source_id)
            .or_default()
            .push(rel_id.clone());

        self.relationships_to
            .entry(target_id)
            .or_default()
            .push(rel_id.clone());

        Ok(rel_id)
    }

    fn get_relationship(&self, relationship_id: &str) -> Option<Relationship> {
        self.relationships
            .get(relationship_id)
            .map(|entry| entry.value().clone())
    }

    fn delete_relationship(&mut self, relationship_id: &str) -> crate::utils::Result<bool> {
        if let Some((_, relationship)) = self.relationships.remove(relationship_id) {
            if let Some(mut rels) = self.relationships_from.get_mut(&relationship.source_id) {
                rels.retain(|id| id != relationship_id);
            }
            if let Some(mut rels) = self.relationships_to.get_mut(&relationship.target_id) {
                rels.retain(|id| id != relationship_id);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn list_relationships(
        &self,
        relationship_type: Option<RelationshipType>,
    ) -> Vec<Relationship> {
        self.relationships
            .iter()
            .map(|entry| entry.value().clone())
            .filter(|rel| {
                if let Some(ref rt) = relationship_type {
                    &rel.relationship_type == rt
                } else {
                    true
                }
            })
            .collect()
    }

    fn get_relationships_from(&self, entity_id: &str) -> Vec<Relationship> {
        self.relationships_from
            .get(entity_id)
            .map(|entry| {
                entry
                    .value()
                    .iter()
                    .filter_map(|rel_id| self.get_relationship(rel_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn get_relationships_to(&self, entity_id: &str) -> Vec<Relationship> {
        self.relationships_to
            .get(entity_id)
            .map(|entry| {
                entry
                    .value()
                    .iter()
                    .filter_map(|rel_id| self.get_relationship(rel_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn find_entities_by_name(&self, name: &str) -> Vec<Entity> {
        let name_lower = name.to_lowercase();
        self.entities
            .iter()
            .map(|entry| entry.value().clone())
            .filter(|entity| entity.name.to_lowercase().contains(&name_lower))
            .collect()
    }

    fn find_entities_by_property(&self, key: &str, value: &serde_json::Value) -> Vec<Entity> {
        self.entities
            .iter()
            .map(|entry| entry.value().clone())
            .filter(|entity| entity.properties.get(key) == Some(value))
            .collect()
    }
}
