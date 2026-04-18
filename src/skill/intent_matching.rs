use crate::skill::{MetadataIndexStore, SkillMetadata, InMemoryMetadataIndexStore};
use crate::utils::Result;
use hashlink::LinkedHashMap;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

#[derive(Debug, Clone)]
pub struct SkillMatch {
    pub skill_id: String,
    pub metadata: SkillMetadata,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchConfig {
    pub threshold: f64,
    pub max_results: usize,
    pub enable_tag_match: bool,
    pub enable_category_match: bool,
    pub enable_keyword_match: bool,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            max_results: 10,
            enable_tag_match: true,
            enable_category_match: true,
            enable_keyword_match: true,
        }
    }
}

#[derive(Clone)]
pub struct IntentMatchingEngine {
    index_store: Arc<InMemoryMetadataIndexStore>,
    config: MatchConfig,
    match_cache: LinkedHashMap<String, Vec<SkillMatch>>,
    cache_capacity: usize,
}

impl Default for IntentMatchingEngine {
    fn default() -> Self {
        Self {
            index_store: Arc::new(crate::skill::InMemoryMetadataIndexStore::new()),
            config: MatchConfig::default(),
            match_cache: LinkedHashMap::new(),
            cache_capacity: 100,
        }
    }
}

impl IntentMatchingEngine {
    pub fn new(
        index_store: Arc<InMemoryMetadataIndexStore>,
        config: MatchConfig,
    ) -> Self {
        Self {
            index_store,
            config,
            match_cache: LinkedHashMap::new(),
            cache_capacity: 100,
        }
    }

    pub fn with_cache_capacity(mut self, capacity: usize) -> Self {
        self.cache_capacity = capacity;
        self
    }

    #[instrument(skip(self, intent), fields(intent = %intent))]
    pub fn match_intent(&self, intent: &str) -> Result<Vec<SkillMatch>> {
        debug!("Matching intent");

        let intent_normalized = intent.trim().to_lowercase();

        if intent_normalized.is_empty() {
            warn!("Empty intent provided for matching");
            return Ok(Vec::new());
        }

        let mut engine_mut = Self {
            index_store: self.index_store.clone(),
            config: self.config.clone(),
            match_cache: self.match_cache.clone(),
            cache_capacity: self.cache_capacity,
        };

        if let Some(cached) = engine_mut.match_cache.get(&intent_normalized) {
            debug!("Cache hit for intent");
            return Ok(cached.clone());
        }

        debug!("Cache miss for intent, calculating matches");

        let all_metadata = self.index_store.list_all()?;
        let mut matches = Vec::new();

        for metadata in all_metadata {
            let score = self.calculate_match_score(intent, &metadata);
            matches.push(SkillMatch {
                skill_id: metadata.id.clone(),
                metadata,
                score,
            });
        }

        let filtered_sorted = self.filter_and_sort(matches);
        let limited = filtered_sorted
            .into_iter()
            .take(self.config.max_results)
            .collect::<Vec<_>>();

        engine_mut
            .match_cache
            .insert(intent_normalized, limited.clone());
        engine_mut.evict_old_cache_entries();

        info!(
            "Successfully matched intent, found {} results",
            limited.len()
        );
        Ok(limited)
    }

    fn evict_old_cache_entries(&mut self) {
        while self.match_cache.len() > self.cache_capacity {
            if let Some((key, _)) = self.match_cache.front() {
                let key = key.clone();
                self.match_cache.remove(&key);
                debug!("Evicted old cache entry");
            }
        }
    }

    #[instrument(skip(self, intent, metadata), fields(intent = %intent, skill_id = %metadata.id))]
    pub fn calculate_match_score(&self, intent: &str, metadata: &SkillMetadata) -> f64 {
        let mut total_score = 0.0;

        if self.config.enable_keyword_match {
            total_score += self.calculate_keyword_score(intent, metadata);
        }

        if self.config.enable_tag_match {
            total_score += self.calculate_tag_score(intent, metadata);
        }

        if self.config.enable_category_match {
            total_score += self.calculate_category_score(intent, metadata);
        }

        debug!(
            "Calculated match score for skill {}: {}",
            metadata.id, total_score
        );

        total_score.min(1.0)
    }

    #[instrument(skip(self, intent, metadata), fields(intent = %intent, skill_id = %metadata.id))]
    pub fn calculate_keyword_score(&self, intent: &str, metadata: &SkillMetadata) -> f64 {
        let intent_lower = intent.to_lowercase();
        let name_lower = metadata.name.to_lowercase();
        let desc_lower = metadata.description.to_lowercase();

        let intent_words: Vec<&str> = intent_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        if intent_words.is_empty() {
            return 0.0;
        }

        let mut match_count = 0;

        for word in &intent_words {
            if name_lower.contains(word) || desc_lower.contains(word) {
                match_count += 1;
            }
        }

        let score = (match_count as f64 / intent_words.len() as f64) * 0.4;

        debug!(
            "Keyword score for skill {}: {} ({} matches out of {} words)",
            metadata.id,
            score,
            match_count,
            intent_words.len()
        );

        score
    }

    #[instrument(skip(self, intent, metadata), fields(intent = %intent, skill_id = %metadata.id))]
    pub fn calculate_tag_score(&self, intent: &str, metadata: &SkillMetadata) -> f64 {
        if metadata.tags.is_empty() {
            return 0.0;
        }

        let intent_lower = intent.to_lowercase();
        let intent_words: Vec<&str> = intent_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        let mut matched_tags = 0;

        for tag in &metadata.tags {
            let tag_lower = tag.to_lowercase();
            if intent_words.iter().any(|&word| word == tag_lower)
                || intent_lower.contains(&tag_lower)
            {
                matched_tags += 1;
            }
        }

        let score = (matched_tags as f64 / metadata.tags.len() as f64) * 0.35;

        debug!(
            "Tag score for skill {}: {} ({} matched tags out of {})",
            metadata.id,
            score,
            matched_tags,
            metadata.tags.len()
        );

        score
    }

    #[instrument(skip(self, intent, metadata), fields(intent = %intent, skill_id = %metadata.id))]
    pub fn calculate_category_score(&self, intent: &str, metadata: &SkillMetadata) -> f64 {
        if metadata.categories.is_empty() {
            return 0.0;
        }

        let intent_lower = intent.to_lowercase();
        let mut has_match = false;

        for category in &metadata.categories {
            if intent_lower.contains(&category.to_lowercase()) {
                has_match = true;
                break;
            }
        }

        let score = if has_match { 0.25 } else { 0.0 };

        debug!(
            "Category score for skill {}: {} (match: {})",
            metadata.id, score, has_match
        );

        score
    }

    #[instrument(skip(self, matches))]
    pub fn filter_and_sort(&self, matches: Vec<SkillMatch>) -> Vec<SkillMatch> {
        let mut filtered: Vec<SkillMatch> = matches
            .into_iter()
            .filter(|m| m.score >= self.config.threshold)
            .collect();

        filtered.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        debug!(
            "Filtered and sorted matches: {} passed threshold",
            filtered.len()
        );

        filtered
    }

    pub fn clear_cache(&mut self) {
        self.match_cache.clear();
        info!("Match cache cleared");
    }

    pub fn cache_size(&self) -> usize {
        self.match_cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{
        CallMode, InMemoryMetadataIndexStore, PermissionLevel, SkillMetadata, SkillPriority,
        Version,
    };
    use chrono::Utc;
    use std::collections::HashMap;

    fn create_test_metadata(
        id: &str,
        name: &str,
        tags: Vec<String>,
        categories: Vec<String>,
    ) -> SkillMetadata {
        SkillMetadata {
            id: id.to_string(),
            name: name.to_string(),
            version: Version::new(1, 0, 0),
            description: format!("{} description", name),
            long_description: None,
            tags,
            categories,
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

    fn setup_test_engine() -> IntentMatchingEngine {
        let mut store = InMemoryMetadataIndexStore::new();

        let metadata1 = create_test_metadata(
            "code-gen-1",
            "Code Generator",
            vec![
                "code".to_string(),
                "generation".to_string(),
                "ai".to_string(),
            ],
            vec!["development".to_string()],
        );

        let metadata2 = create_test_metadata(
            "data-parser-1",
            "Data Parser",
            vec![
                "data".to_string(),
                "parsing".to_string(),
                "analysis".to_string(),
            ],
            vec!["data".to_string()],
        );

        let metadata3 = create_test_metadata(
            "email-composer-1",
            "Email Composer",
            vec!["email".to_string(), "communication".to_string()],
            vec!["productivity".to_string()],
        );

        store.index_metadata(metadata1).unwrap();
        store.index_metadata(metadata2).unwrap();
        store.index_metadata(metadata3).unwrap();

        IntentMatchingEngine::new(Arc::new(store), MatchConfig::default())
    }

    #[test]
    fn test_new_engine() {
        let store = InMemoryMetadataIndexStore::new();
        let engine = IntentMatchingEngine::new(Arc::new(store), MatchConfig::default());
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_match_config_default() {
        let config = MatchConfig::default();
        assert_eq!(config.threshold, 0.5);
        assert_eq!(config.max_results, 10);
        assert!(config.enable_tag_match);
        assert!(config.enable_category_match);
        assert!(config.enable_keyword_match);
    }

    #[test]
    fn test_keyword_score() {
        let engine = IntentMatchingEngine::default();
        let metadata = create_test_metadata(
            "test-1",
            "Code Generator",
            vec!["code".to_string()],
            vec!["development".to_string()],
        );

        let score = engine.calculate_keyword_score("generate code", &metadata);
        assert!(score > 0.0);
        assert!(score <= 0.4);
    }

    #[test]
    fn test_tag_score() {
        let engine = IntentMatchingEngine::default();
        let metadata = create_test_metadata(
            "test-1",
            "Code Generator",
            vec!["code".to_string(), "ai".to_string()],
            vec!["development".to_string()],
        );

        let score = engine.calculate_tag_score("code ai", &metadata);
        assert!(score > 0.0);
        assert!(score <= 0.35);
    }

    #[test]
    fn test_category_score() {
        let engine = IntentMatchingEngine::default();
        let metadata = create_test_metadata(
            "test-1",
            "Code Generator",
            vec!["code".to_string()],
            vec!["development".to_string()],
        );

        let score = engine.calculate_category_score("development tools", &metadata);
        assert_eq!(score, 0.25);
    }

    #[test]
    fn test_total_match_score() {
        let engine = IntentMatchingEngine::default();
        let metadata = create_test_metadata(
            "test-1",
            "Code Generator",
            vec!["code".to_string(), "ai".to_string()],
            vec!["development".to_string()],
        );

        let score = engine.calculate_match_score("code ai development", &metadata);
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_match_intent() {
        let engine = setup_test_engine();
        let results = engine.match_intent("code generation").unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].skill_id, "code-gen-1");
    }

    #[test]
    fn test_filter_and_sort() {
        let engine = IntentMatchingEngine::default();
        let metadata1 = create_test_metadata("test-1", "Test 1", vec![], vec![]);
        let metadata2 = create_test_metadata("test-2", "Test 2", vec![], vec![]);

        let matches = vec![
            SkillMatch {
                skill_id: "test-1".to_string(),
                metadata: metadata1,
                score: 0.3,
            },
            SkillMatch {
                skill_id: "test-2".to_string(),
                metadata: metadata2,
                score: 0.8,
            },
        ];

        let filtered = engine.filter_and_sort(matches);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].skill_id, "test-2");
    }

    #[test]
    fn test_clear_cache() {
        let mut engine = setup_test_engine();
        engine.match_intent("code").unwrap();
        assert_eq!(engine.cache_size(), 1);
        engine.clear_cache();
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_empty_intent() {
        let engine = setup_test_engine();
        let results = engine.match_intent("").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_custom_config() {
        let store = InMemoryMetadataIndexStore::new();
        let config = MatchConfig {
            threshold: 0.7,
            max_results: 5,
            enable_tag_match: false,
            enable_category_match: false,
            enable_keyword_match: true,
        };
        let engine = IntentMatchingEngine::new(Arc::new(store), config);
        assert_eq!(engine.config.threshold, 0.7);
        assert_eq!(engine.config.max_results, 5);
        assert!(!engine.config.enable_tag_match);
    }
}
