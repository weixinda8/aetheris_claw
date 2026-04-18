use super::*;
use std::sync::Arc;

pub struct SemanticSearchEngine {
    graph: Arc<InMemoryKnowledgeGraph>,
}

impl SemanticSearchEngine {
    pub fn new(graph: Arc<InMemoryKnowledgeGraph>) -> Self {
        Self { graph }
    }

    pub fn search_by_keywords(&self, keywords: &[String]) -> Vec<Entity> {
        let mut results = std::collections::HashSet::new();

        for keyword in keywords {
            let entities = self.graph.find_entities_by_name(keyword);
            for entity in entities {
                results.insert(entity.id.clone());
            }
        }

        results
            .iter()
            .filter_map(|id| self.graph.get_entity(id))
            .collect()
    }

    pub fn search_similar_entities(
        &self,
        entity_id: &str,
        max_depth: usize,
    ) -> Vec<(Entity, f64)> {
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        let mut results = Vec::new();

        if let Some(_start_entity) = self.graph.get_entity(entity_id) {
            visited.insert(entity_id.to_string());
            queue.push_back((entity_id.to_string(), 0));

            while let Some((current_id, depth)) = queue.pop_front() {
                if depth >= max_depth {
                    continue;
                }

                let rels = self.graph.get_relationships_from(&current_id);
                for rel in rels {
                    if !visited.contains(&rel.target_id) {
                        visited.insert(rel.target_id.clone());
                        queue.push_back((rel.target_id.clone(), depth + 1));

                        if let Some(entity) = self.graph.get_entity(&rel.target_id) {
                            let similarity = 1.0 / (depth + 1) as f64;
                            results.push((entity, similarity));
                        }
                    }
                }

                let rels_to = self.graph.get_relationships_to(&current_id);
                for rel in rels_to {
                    if !visited.contains(&rel.source_id) {
                        visited.insert(rel.source_id.clone());
                        queue.push_back((rel.source_id.clone(), depth + 1));

                        if let Some(entity) = self.graph.get_entity(&rel.source_id) {
                            let similarity = 1.0 / (depth + 1) as f64;
                            results.push((entity, similarity));
                        }
                    }
                }
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    pub fn find_path(
        &self,
        start_id: &str,
        end_id: &str,
        max_depth: usize,
    ) -> Option<Vec<Entity>> {
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        queue.push_back((vec![start_id.to_string()], 0));
        visited.insert(start_id.to_string());

        while let Some((path, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            let current_id = path.last().unwrap();

            if current_id == end_id {
                let entity_path: Vec<_> = path
                    .iter()
                    .filter_map(|id| self.graph.get_entity(id))
                    .collect();
                return Some(entity_path);
            }

            let rels = self.graph.get_relationships_from(current_id);
            for rel in rels {
                if !visited.contains(&rel.target_id) {
                    visited.insert(rel.target_id.clone());
                    let mut new_path = path.clone();
                    new_path.push(rel.target_id.clone());
                    queue.push_back((new_path, depth + 1));
                }
            }

            let rels_to = self.graph.get_relationships_to(current_id);
            for rel in rels_to {
                if !visited.contains(&rel.source_id) {
                    visited.insert(rel.source_id.clone());
                    let mut new_path = path.clone();
                    new_path.push(rel.source_id.clone());
                    queue.push_back((new_path, depth + 1));
                }
            }
        }

        None
    }

    pub fn find_connected_components(
        &self,
        entity_id: &str,
        relationship_types: Option<&[RelationshipType]>,
    ) -> Vec<Entity> {
        let mut visited = std::collections::HashSet::new();
        let mut result = Vec::new();
        let mut stack = vec![entity_id.to_string()];

        visited.insert(entity_id.to_string());

        while let Some(current_id) = stack.pop() {
            if let Some(entity) = self.graph.get_entity(&current_id) {
                result.push(entity);
            }

            let rels = self.graph.get_relationships_from(&current_id);
            for rel in rels {
                let should_include = relationship_types
                    .map(|types| types.contains(&rel.relationship_type))
                    .unwrap_or(true);

                if should_include && !visited.contains(&rel.target_id) {
                    visited.insert(rel.target_id.clone());
                    stack.push(rel.target_id.clone());
                }
            }

            let rels_to = self.graph.get_relationships_to(&current_id);
            for rel in rels_to {
                let should_include = relationship_types
                    .map(|types| types.contains(&rel.relationship_type))
                    .unwrap_or(true);

                if should_include && !visited.contains(&rel.source_id) {
                    visited.insert(rel.source_id.clone());
                    stack.push(rel.source_id.clone());
                }
            }
        }

        result
    }

    pub fn find_neighbors(
        &self,
        entity_id: &str,
        relationship_type: Option<RelationshipType>,
    ) -> Vec<(Entity, Relationship)> {
        let mut neighbors = Vec::new();

        let rels = self.graph.get_relationships_from(entity_id);
        for rel in rels {
            let match_type = relationship_type
                .as_ref()
                .map(|rt| rt == &rel.relationship_type)
                .unwrap_or(true);

            if match_type {
                if let Some(entity) = self.graph.get_entity(&rel.target_id) {
                    neighbors.push((entity, rel));
                }
            }
        }

        let rels_to = self.graph.get_relationships_to(entity_id);
        for rel in rels_to {
            let match_type = relationship_type
                .as_ref()
                .map(|rt| rt == &rel.relationship_type)
                .unwrap_or(true);

            if match_type {
                if let Some(entity) = self.graph.get_entity(&rel.source_id) {
                    neighbors.push((entity, rel));
                }
            }
        }

        neighbors
    }
}
