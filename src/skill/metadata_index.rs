use crate::skill::SkillMetadata;
use crate::utils::Result;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

pub trait MetadataIndexStore: Send + Sync {
    fn index_metadata(&self, metadata: SkillMetadata) -> Result<()>;
    fn remove_metadata(&self, skill_id: &str) -> Result<()>;
    fn get_metadata(&self, skill_id: &str) -> Result<Option<SkillMetadata>>;
    fn search_by_name(&self, query: &str) -> Result<Vec<SkillMetadata>>;
    fn search_by_tags(&self, tags: &[String]) -> Result<Vec<SkillMetadata>>;
    fn search_by_category(&self, category: &str) -> Result<Vec<SkillMetadata>>;
    fn update_metadata(&self, metadata: SkillMetadata) -> Result<()>;
    fn rebuild_index(&self, metadatas: Vec<SkillMetadata>) -> Result<()>;
    fn list_all(&self) -> Result<Vec<SkillMetadata>>;
}

#[derive(Debug, Clone)]
pub struct InMemoryMetadataIndexStore {
    metadata_store: Arc<DashMap<String, SkillMetadata>>,
    name_index: Arc<DashMap<String, Vec<String>>>,
    tag_index: Arc<DashMap<String, Vec<String>>>,
    category_index: Arc<DashMap<String, Vec<String>>>,
}

impl Default for InMemoryMetadataIndexStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryMetadataIndexStore {
    pub fn new() -> Self {
        Self {
            metadata_store: Arc::new(DashMap::new()),
            name_index: Arc::new(DashMap::new()),
            tag_index: Arc::new(DashMap::new()),
            category_index: Arc::new(DashMap::new()),
        }
    }

    fn add_to_name_index(&self, skill_id: &str, name: &str) {
        let normalized_name = name.to_lowercase();
        let mut entry = self.name_index.entry(normalized_name).or_default();
        if !entry.contains(&skill_id.to_string()) {
            entry.push(skill_id.to_string());
        }
    }

    fn add_to_tag_index(&self, skill_id: &str, tags: &[String]) {
        for tag in tags {
            let normalized_tag = tag.to_lowercase();
            let mut entry = self.tag_index.entry(normalized_tag).or_default();
            if !entry.contains(&skill_id.to_string()) {
                entry.push(skill_id.to_string());
            }
        }
    }

    fn add_to_category_index(&self, skill_id: &str, categories: &[String]) {
        for category in categories {
            let normalized_category = category.to_lowercase();
            let mut entry = self.category_index.entry(normalized_category).or_default();
            if !entry.contains(&skill_id.to_string()) {
                entry.push(skill_id.to_string());
            }
        }
    }

    fn remove_from_name_index(&self, skill_id: &str, name: &str) {
        let normalized_name = name.to_lowercase();
        if let Some(mut entry) = self.name_index.get_mut(&normalized_name) {
            entry.retain(|id| id != skill_id);
            if entry.is_empty() {
                drop(entry);
                self.name_index.remove(&normalized_name);
            }
        }
    }

    fn remove_from_tag_index(&self, skill_id: &str, tags: &[String]) {
        for tag in tags {
            let normalized_tag = tag.to_lowercase();
            if let Some(mut entry) = self.tag_index.get_mut(&normalized_tag) {
                entry.retain(|id| id != skill_id);
                if entry.is_empty() {
                    drop(entry);
                    self.tag_index.remove(&normalized_tag);
                }
            }
        }
    }

    fn remove_from_category_index(&self, skill_id: &str, categories: &[String]) {
        for category in categories {
            let normalized_category = category.to_lowercase();
            if let Some(mut entry) = self.category_index.get_mut(&normalized_category) {
                entry.retain(|id| id != skill_id);
                if entry.is_empty() {
                    drop(entry);
                    self.category_index.remove(&normalized_category);
                }
            }
        }
    }
}

impl MetadataIndexStore for InMemoryMetadataIndexStore {
    #[instrument(skip(self, metadata), fields(skill_id = %metadata.id, name = %metadata.name))]
    fn index_metadata(&self, metadata: SkillMetadata) -> Result<()> {
        debug!("Indexing skill metadata");

        let skill_id = metadata.id.clone();
        let name = metadata.name.clone();
        let tags = metadata.tags.clone();
        let categories = metadata.categories.clone();

        self.metadata_store.insert(skill_id.clone(), metadata);

        self.add_to_name_index(&skill_id, &name);
        self.add_to_tag_index(&skill_id, &tags);
        self.add_to_category_index(&skill_id, &categories);

        info!("Successfully indexed skill metadata");
        Ok(())
    }

    #[instrument(skip(self), fields(skill_id = %skill_id))]
    fn remove_metadata(&self, skill_id: &str) -> Result<()> {
        debug!("Removing skill metadata from index");

        if let Some((_, metadata)) = self.metadata_store.remove(skill_id) {
            self.remove_from_name_index(skill_id, &metadata.name);
            self.remove_from_tag_index(skill_id, &metadata.tags);
            self.remove_from_category_index(skill_id, &metadata.categories);
            info!("Successfully removed skill metadata from index");
        } else {
            warn!("Skill metadata not found in index for removal");
        }

        Ok(())
    }

    #[instrument(skip(self), fields(skill_id = %skill_id))]
    fn get_metadata(&self, skill_id: &str) -> Result<Option<SkillMetadata>> {
        debug!("Getting skill metadata");

        let result = self.metadata_store.get(skill_id).map(|entry| entry.clone());

        if result.is_some() {
            debug!("Found skill metadata");
        } else {
            debug!("Skill metadata not found");
        }

        Ok(result)
    }

    #[instrument(skip(self), fields(query = %query))]
    fn search_by_name(&self, query: &str) -> Result<Vec<SkillMetadata>> {
        debug!("Searching skills by name");

        let normalized_query = query.to_lowercase();
        let mut results = Vec::new();

        for entry in self.name_index.iter() {
            let name = entry.key();
            if name.contains(&normalized_query) {
                let skill_ids = entry.value();
                for skill_id in skill_ids {
                    if let Some(metadata) = self.metadata_store.get(skill_id) {
                        results.push(metadata.clone());
                    }
                }
            }
        }

        info!("Found {} skills by name search", results.len());
        Ok(results)
    }

    #[instrument(skip(self), fields(tags = ?tags))]
    fn search_by_tags(&self, tags: &[String]) -> Result<Vec<SkillMetadata>> {
        debug!("Searching skills by tags");

        if tags.is_empty() {
            debug!("No tags provided for search");
            return Ok(Vec::new());
        }

        let mut results = Vec::new();

        for tag in tags {
            let normalized_tag = tag.to_lowercase();
            if let Some(skill_ids) = self.tag_index.get(&normalized_tag) {
                for skill_id in skill_ids.iter() {
                    if let Some(metadata) = self.metadata_store.get(skill_id) {
                        if !results.iter().any(|m: &SkillMetadata| m.id == metadata.id) {
                            results.push(metadata.clone());
                        }
                    }
                }
            }
        }

        info!("Found {} skills by tags search", results.len());
        Ok(results)
    }

    #[instrument(skip(self), fields(category = %category))]
    fn search_by_category(&self, category: &str) -> Result<Vec<SkillMetadata>> {
        debug!("Searching skills by category");

        let normalized_category = category.to_lowercase();
        let mut results = Vec::new();

        if let Some(skill_ids) = self.category_index.get(&normalized_category) {
            for skill_id in skill_ids.iter() {
                if let Some(metadata) = self.metadata_store.get(skill_id) {
                    results.push(metadata.clone());
                }
            }
        }

        info!("Found {} skills by category search", results.len());
        Ok(results)
    }

    #[instrument(skip(self, metadata), fields(skill_id = %metadata.id, name = %metadata.name))]
    fn update_metadata(&self, metadata: SkillMetadata) -> Result<()> {
        debug!("Updating skill metadata");

        let skill_id = metadata.id.clone();

        if let Some(old_metadata) = self.metadata_store.get(&skill_id) {
            self.remove_from_name_index(&skill_id, &old_metadata.name);
            self.remove_from_tag_index(&skill_id, &old_metadata.tags);
            self.remove_from_category_index(&skill_id, &old_metadata.categories);
        }

        self.metadata_store
            .insert(skill_id.clone(), metadata.clone());
        self.add_to_name_index(&skill_id, &metadata.name);
        self.add_to_tag_index(&skill_id, &metadata.tags);
        self.add_to_category_index(&skill_id, &metadata.categories);

        info!("Successfully updated skill metadata");
        Ok(())
    }

    #[instrument(skip(self, metadatas), fields(count = %metadatas.len()))]
    fn rebuild_index(&self, metadatas: Vec<SkillMetadata>) -> Result<()> {
        info!("Rebuilding metadata index");

        self.metadata_store.clear();
        self.name_index.clear();
        self.tag_index.clear();
        self.category_index.clear();

        for metadata in metadatas {
            let skill_id = metadata.id.clone();
            let name = metadata.name.clone();
            let tags = metadata.tags.clone();
            let categories = metadata.categories.clone();

            self.metadata_store.insert(skill_id.clone(), metadata);
            self.add_to_name_index(&skill_id, &name);
            self.add_to_tag_index(&skill_id, &tags);
            self.add_to_category_index(&skill_id, &categories);
        }

        info!("Successfully rebuilt metadata index");
        Ok(())
    }

    #[instrument(skip(self))]
    fn list_all(&self) -> Result<Vec<SkillMetadata>> {
        debug!("Listing all skill metadata");

        let results: Vec<SkillMetadata> = self
            .metadata_store
            .iter()
            .map(|entry| entry.clone())
            .collect();

        info!("Found {} skills in index", results.len());
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{CallMode, PermissionLevel, SkillMetadata, SkillPriority, Version};
    use chrono::Utc;
    use std::collections::HashMap;

    fn create_test_metadata(id: &str, name: &str) -> SkillMetadata {
        SkillMetadata {
            id: id.to_string(),
            name: name.to_string(),
            version: Version::new(1, 0, 0),
            description: "Test skill".to_string(),
            long_description: None,
            tags: vec!["test".to_string(), "sample".to_string()],
            categories: vec!["development".to_string()],
            author: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            call_mode: CallMode::Text,
            permission_level: PermissionLevel::Public,
            priority: SkillPriority::Medium,
            required_permissions: Vec::new(),
            input_schema: None,
            output_schema: None,
            example_input: None,
            example_output: None,
            dependencies: Vec::new(),
            is_active: true,
            is_deprecated: false,
            deprecation_reason: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_index_and_get_metadata() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata = create_test_metadata("test-1", "Test Skill");

        store.index_metadata(metadata.clone()).unwrap();

        let retrieved = store.get_metadata("test-1").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test-1");
    }

    #[test]
    fn test_remove_metadata() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata = create_test_metadata("test-1", "Test Skill");

        store.index_metadata(metadata.clone()).unwrap();
        assert!(store.get_metadata("test-1").unwrap().is_some());

        store.remove_metadata("test-1").unwrap();
        assert!(store.get_metadata("test-1").unwrap().is_none());
    }

    #[test]
    fn test_search_by_name() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata1 = create_test_metadata("test-1", "Code Generator");
        let metadata2 = create_test_metadata("test-2", "Data Parser");

        store.index_metadata(metadata1).unwrap();
        store.index_metadata(metadata2).unwrap();

        let results = store.search_by_name("code").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test-1");
    }

    #[test]
    fn test_search_by_tags() {
        let mut store = InMemoryMetadataIndexStore::new();
        let mut metadata = create_test_metadata("test-1", "Test Skill");
        metadata.tags = vec!["ai".to_string(), "machine-learning".to_string()];

        store.index_metadata(metadata).unwrap();

        let results = store.search_by_tags(&["ai".to_string()]).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_by_category() {
        let mut store = InMemoryMetadataIndexStore::new();
        let mut metadata = create_test_metadata("test-1", "Test Skill");
        metadata.categories = vec!["productivity".to_string()];

        store.index_metadata(metadata).unwrap();

        let results = store.search_by_category("productivity").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_update_metadata() {
        let mut store = InMemoryMetadataIndexStore::new();
        let mut metadata = create_test_metadata("test-1", "Old Name");

        store.index_metadata(metadata.clone()).unwrap();

        metadata.name = "New Name".to_string();
        store.update_metadata(metadata.clone()).unwrap();

        let retrieved = store.get_metadata("test-1").unwrap().unwrap();
        assert_eq!(retrieved.name, "New Name");

        let results = store.search_by_name("new").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_rebuild_index() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata1 = create_test_metadata("test-1", "Skill 1");
        let metadata2 = create_test_metadata("test-2", "Skill 2");

        store.index_metadata(metadata1).unwrap();
        assert_eq!(store.list_all().unwrap().len(), 1);

        let new_metadatas = vec![metadata2];
        store.rebuild_index(new_metadatas).unwrap();

        let all = store.list_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "test-2");
    }

    #[test]
    fn test_list_all() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata1 = create_test_metadata("test-1", "Skill 1");
        let metadata2 = create_test_metadata("test-2", "Skill 2");

        store.index_metadata(metadata1).unwrap();
        store.index_metadata(metadata2).unwrap();

        let all = store.list_all().unwrap();
        assert_eq!(all.len(), 2);
    }
}
