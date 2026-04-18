use crate::skill::{Skill, SkillLoader};
use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct SubSkillRelationship {
    pub parent_skill_id: String,
    pub sub_skill_id: String,
    pub depth: usize,
}

#[derive(Debug)]
pub struct SubSkillManager {
    skill_loader: SkillLoader,
    sub_skill_relationships: DashMap<String, Vec<SubSkillRelationship>>,
    parent_to_sub_skills: DashMap<String, Vec<String>>,
    sub_skill_to_parent: DashMap<String, String>,
}

impl SubSkillManager {
    pub fn new() -> Self {
        Self {
            skill_loader: SkillLoader::new(),
            sub_skill_relationships: DashMap::new(),
            parent_to_sub_skills: DashMap::new(),
            sub_skill_to_parent: DashMap::new(),
        }
    }

    pub fn with_skill_loader(skill_loader: SkillLoader) -> Self {
        Self {
            skill_loader,
            sub_skill_relationships: DashMap::new(),
            parent_to_sub_skills: DashMap::new(),
            sub_skill_to_parent: DashMap::new(),
        }
    }

    pub async fn load_from_directory(&self, dir_path: &str) -> Result<Vec<Arc<dyn Skill>>> {
        info!("Loading sub-skills from directory: {}", dir_path);

        let mut visited = HashSet::new();
        let mut all_skills = Vec::new();
        let mut skill_queue = VecDeque::new();

        let root_skills = self.skill_loader.load_from_path(dir_path).await?;
        for skill in root_skills {
            skill_queue.push_back((skill.clone(), 0));
            all_skills.push(skill);
        }

        while let Some((current_skill, depth)) = skill_queue.pop_front() {
            let skill_metadata = current_skill.metadata();
            let skill_id = skill_metadata.id.clone();

            if visited.contains(&skill_id) {
                continue;
            }
            visited.insert(skill_id.clone());

            let skill_path = Path::new(dir_path);
            let sub_skills_dir = skill_path.join(&skill_id).join("sub-skills");

            if sub_skills_dir.exists() && sub_skills_dir.is_dir() {
                debug!(
                    "Loading sub-skills for skill: {} at depth: {}",
                    skill_id, depth
                );

                let sub_skills = self
                    .skill_loader
                    .load_from_path(sub_skills_dir.to_str().unwrap())
                    .await?;

                for sub_skill in sub_skills {
                    let sub_skill_id = sub_skill.metadata().id.clone();

                    let relationship = SubSkillRelationship {
                        parent_skill_id: skill_id.clone(),
                        sub_skill_id: sub_skill_id.clone(),
                        depth: depth + 1,
                    };

                    self.sub_skill_relationships
                        .entry(skill_id.clone())
                        .or_default()
                        .push(relationship);

                    self.parent_to_sub_skills
                        .entry(skill_id.clone())
                        .or_default()
                        .push(sub_skill_id.clone());

                    self.sub_skill_to_parent
                        .insert(sub_skill_id.clone(), skill_id.clone());

                    skill_queue.push_back((sub_skill.clone(), depth + 1));
                    all_skills.push(sub_skill);
                }
            }
        }

        info!(
            "Loaded {} total skills (including sub-skills)",
            all_skills.len()
        );
        Ok(all_skills)
    }

    pub fn list_sub_skills(&self, parent_skill_id: &str) -> Vec<String> {
        self.parent_to_sub_skills
            .get(parent_skill_id)
            .map(|sub_skills| sub_skills.clone())
            .unwrap_or_default()
    }

    pub fn get_sub_skill(
        &self,
        parent_skill_id: &str,
        sub_skill_id: &str,
    ) -> Option<SubSkillRelationship> {
        self.sub_skill_relationships
            .get(parent_skill_id)
            .and_then(|relationships| {
                relationships
                    .iter()
                    .find(|rel| rel.sub_skill_id == sub_skill_id)
                    .cloned()
            })
    }

    pub fn get_parent_skill(&self, sub_skill_id: &str) -> Option<String> {
        self.sub_skill_to_parent
            .get(sub_skill_id)
            .map(|parent_id| parent_id.clone())
    }

    pub fn is_sub_skill(&self, skill_id: &str) -> bool {
        self.sub_skill_to_parent.contains_key(skill_id)
    }

    pub fn get_sub_skill_depth(&self, skill_id: &str) -> Option<usize> {
        self.sub_skill_to_parent
            .get(skill_id)
            .and_then(|parent_id_ref| {
                let parent_id = parent_id_ref.clone();
                self.sub_skill_relationships
                    .get(&parent_id)
                    .and_then(|relationships| {
                        relationships
                            .iter()
                            .find(|rel| rel.sub_skill_id == skill_id)
                            .map(|rel| rel.depth)
                    })
            })
    }

    pub async fn call_sub_skill(
        &self,
        registry: &crate::skill::SkillRegistry,
        parent_skill_id: &str,
        sub_skill_id: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        debug!(
            "Calling sub-skill: {} from parent: {}",
            sub_skill_id, parent_skill_id
        );

        if !self.is_sub_skill(sub_skill_id) {
            return Err(AetherisError::Skill(format!(
                "Skill {} is not a sub-skill of {}",
                sub_skill_id, parent_skill_id
            )));
        }

        if let Some(parent_id) = self.get_parent_skill(sub_skill_id) {
            if parent_id != parent_skill_id {
                return Err(AetherisError::Skill(format!(
                    "Skill {} is a sub-skill of {}, not {}",
                    sub_skill_id, parent_id, parent_skill_id
                )));
            }
        }

        let skill = registry.get(sub_skill_id).ok_or_else(|| {
            AetherisError::Skill(format!("Sub-skill not found: {}", sub_skill_id))
        })?;

        skill.execute(input).await
    }

    pub fn clear(&self) {
        debug!("Clearing sub-skill manager state");
        self.sub_skill_relationships.clear();
        self.parent_to_sub_skills.clear();
        self.sub_skill_to_parent.clear();
    }

    pub fn relationship_count(&self) -> usize {
        self.sub_skill_relationships
            .iter()
            .map(|entry| entry.value().len())
            .sum()
    }
}

impl Default for SubSkillManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{BaseSkill, SkillMetadata, Version};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_sub_skill_manager_new() {
        let manager = SubSkillManager::new();
        assert_eq!(manager.relationship_count(), 0);
    }

    #[test]
    fn test_list_sub_skills_empty() {
        let manager = SubSkillManager::new();
        let sub_skills = manager.list_sub_skills("non-existent");
        assert!(sub_skills.is_empty());
    }

    #[test]
    fn test_is_sub_skill_empty() {
        let manager = SubSkillManager::new();
        assert!(!manager.is_sub_skill("test"));
    }

    #[test]
    fn test_get_parent_skill_empty() {
        let manager = SubSkillManager::new();
        assert!(manager.get_parent_skill("test").is_none());
    }

    #[test]
    fn test_clear() {
        let manager = SubSkillManager::new();
        manager.clear();
        assert_eq!(manager.relationship_count(), 0);
    }
}
