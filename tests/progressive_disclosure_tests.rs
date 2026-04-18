use aetheris::skill::*;
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

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

fn create_test_skill(
    id: &str,
    name: &str,
    tags: Vec<String>,
    categories: Vec<String>,
) -> Arc<dyn Skill> {
    let metadata = create_test_metadata(id, name, tags, categories);
    BaseSkill::new_arc(metadata)
}

mod metadata_index_tests {
    use super::*;

    #[test]
    fn test_index_and_get_metadata() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata = create_test_metadata("test-1", "Test Skill", vec![], vec![]);

        store.index_metadata(metadata.clone()).unwrap();

        let retrieved = store.get_metadata("test-1").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test-1");
    }

    #[test]
    fn test_remove_metadata() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata = create_test_metadata("test-1", "Test Skill", vec![], vec![]);

        store.index_metadata(metadata.clone()).unwrap();
        assert!(store.get_metadata("test-1").unwrap().is_some());

        store.remove_metadata("test-1").unwrap();
        assert!(store.get_metadata("test-1").unwrap().is_none());
    }

    #[test]
    fn test_remove_nonexistent_metadata() {
        let mut store = InMemoryMetadataIndexStore::new();
        let result = store.remove_metadata("nonexistent");
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_by_name() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata1 = create_test_metadata("test-1", "Code Generator", vec![], vec![]);
        let metadata2 = create_test_metadata("test-2", "Data Parser", vec![], vec![]);

        store.index_metadata(metadata1).unwrap();
        store.index_metadata(metadata2).unwrap();

        let results = store.search_by_name("code").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test-1");
    }

    #[test]
    fn test_search_by_name_case_insensitive() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata = create_test_metadata("test-1", "Code Generator", vec![], vec![]);

        store.index_metadata(metadata).unwrap();

        let results1 = store.search_by_name("CODE").unwrap();
        let results2 = store.search_by_name("code").unwrap();
        assert_eq!(results1.len(), 1);
        assert_eq!(results2.len(), 1);
    }

    #[test]
    fn test_search_by_tags() {
        let mut store = InMemoryMetadataIndexStore::new();
        let mut metadata = create_test_metadata("test-1", "Test Skill", vec![], vec![]);
        metadata.tags = vec!["ai".to_string(), "machine-learning".to_string()];

        store.index_metadata(metadata).unwrap();

        let results = store.search_by_tags(&["ai".to_string()]).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_by_multiple_tags() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata1 = create_test_metadata("test-1", "AI Skill", vec!["ai".to_string()], vec![]);
        let metadata2 = create_test_metadata(
            "test-2",
            "ML Skill",
            vec!["machine-learning".to_string()],
            vec![],
        );
        let metadata3 = create_test_metadata(
            "test-3",
            "Both Skill",
            vec!["ai".to_string(), "machine-learning".to_string()],
            vec![],
        );

        store.index_metadata(metadata1).unwrap();
        store.index_metadata(metadata2).unwrap();
        store.index_metadata(metadata3).unwrap();

        let results = store
            .search_by_tags(&["ai".to_string(), "machine-learning".to_string()])
            .unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_by_empty_tags() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata =
            create_test_metadata("test-1", "Test Skill", vec!["test".to_string()], vec![]);

        store.index_metadata(metadata).unwrap();

        let results = store.search_by_tags(&[]).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_by_category() {
        let mut store = InMemoryMetadataIndexStore::new();
        let mut metadata = create_test_metadata("test-1", "Test Skill", vec![], vec![]);
        metadata.categories = vec!["productivity".to_string()];

        store.index_metadata(metadata).unwrap();

        let results = store.search_by_category("productivity").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_by_nonexistent_category() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata = create_test_metadata(
            "test-1",
            "Test Skill",
            vec![],
            vec!["productivity".to_string()],
        );

        store.index_metadata(metadata).unwrap();

        let results = store.search_by_category("nonexistent").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_update_metadata() {
        let mut store = InMemoryMetadataIndexStore::new();
        let mut metadata = create_test_metadata("test-1", "Old Name", vec![], vec![]);

        store.index_metadata(metadata.clone()).unwrap();

        metadata.name = "New Name".to_string();
        store.update_metadata(metadata.clone()).unwrap();

        let retrieved = store.get_metadata("test-1").unwrap().unwrap();
        assert_eq!(retrieved.name, "New Name");

        let results = store.search_by_name("new").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_update_nonexistent_metadata() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata = create_test_metadata("test-1", "Test Skill", vec![], vec![]);

        let result = store.update_metadata(metadata.clone());
        assert!(result.is_ok());

        let retrieved = store.get_metadata("test-1").unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_rebuild_index() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata1 = create_test_metadata("test-1", "Skill 1", vec![], vec![]);
        let metadata2 = create_test_metadata("test-2", "Skill 2", vec![], vec![]);

        store.index_metadata(metadata1).unwrap();
        assert_eq!(store.list_all().unwrap().len(), 1);

        let new_metadatas = vec![metadata2];
        store.rebuild_index(new_metadatas).unwrap();

        let all = store.list_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "test-2");
    }

    #[test]
    fn test_rebuild_empty_index() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata = create_test_metadata("test-1", "Skill 1", vec![], vec![]);

        store.index_metadata(metadata).unwrap();
        assert_eq!(store.list_all().unwrap().len(), 1);

        store.rebuild_index(vec![]).unwrap();
        assert!(store.list_all().unwrap().is_empty());
    }

    #[test]
    fn test_list_all() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata1 = create_test_metadata("test-1", "Skill 1", vec![], vec![]);
        let metadata2 = create_test_metadata("test-2", "Skill 2", vec![], vec![]);

        store.index_metadata(metadata1).unwrap();
        store.index_metadata(metadata2).unwrap();

        let all = store.list_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_list_empty() {
        let store = InMemoryMetadataIndexStore::new();
        let all = store.list_all().unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn test_get_nonexistent_metadata() {
        let store = InMemoryMetadataIndexStore::new();
        let result = store.get_metadata("nonexistent").unwrap();
        assert!(result.is_none());
    }
}

mod intent_matching_tests {
    use super::*;

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
    fn test_keyword_score_empty_intent() {
        let engine = IntentMatchingEngine::default();
        let metadata = create_test_metadata("test-1", "Test", vec![], vec![]);
        let score = engine.calculate_keyword_score("", &metadata);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_keyword_score_short_words() {
        let engine = IntentMatchingEngine::default();
        let metadata = create_test_metadata("test-1", "Test", vec![], vec![]);
        let score = engine.calculate_keyword_score("a b c", &metadata);
        assert_eq!(score, 0.0);
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
    fn test_tag_score_empty_tags() {
        let engine = IntentMatchingEngine::default();
        let metadata = create_test_metadata("test-1", "Test", vec![], vec![]);
        let score = engine.calculate_tag_score("test", &metadata);
        assert_eq!(score, 0.0);
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
    fn test_category_score_empty_categories() {
        let engine = IntentMatchingEngine::default();
        let metadata = create_test_metadata("test-1", "Test", vec![], vec![]);
        let score = engine.calculate_category_score("test", &metadata);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_category_score_no_match() {
        let engine = IntentMatchingEngine::default();
        let metadata =
            create_test_metadata("test-1", "Test", vec![], vec!["development".to_string()]);
        let score = engine.calculate_category_score("productivity", &metadata);
        assert_eq!(score, 0.0);
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
    fn test_total_match_score_max() {
        let engine = IntentMatchingEngine::default();
        let metadata = create_test_metadata(
            "test-1",
            "Test Skill",
            vec!["test".to_string(), "skill".to_string()],
            vec!["test".to_string()],
        );

        let score = engine.calculate_match_score("test skill test", &metadata);
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
    fn test_match_intent_data() {
        let engine = setup_test_engine();
        let results = engine.match_intent("parse data").unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].skill_id, "data-parser-1");
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
    fn test_filter_and_sort_all_below_threshold() {
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
                score: 0.4,
            },
        ];

        let filtered = engine.filter_and_sort(matches);
        assert!(filtered.is_empty());
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
    fn test_whitespace_intent() {
        let engine = setup_test_engine();
        let results = engine.match_intent("   ").unwrap();
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

    #[test]
    fn test_cache_eviction() {
        let store = InMemoryMetadataIndexStore::new();
        let mut engine = IntentMatchingEngine::new(Arc::new(store), MatchConfig::default())
            .with_cache_capacity(2);

        engine.match_intent("query 1").unwrap();
        engine.match_intent("query 2").unwrap();
        engine.match_intent("query 3").unwrap();

        assert_eq!(engine.cache_size(), 2);
    }

    #[test]
    fn test_match_accuracy() {
        let mut store = InMemoryMetadataIndexStore::new();

        let test_cases = vec![
            (
                "code-gen-1",
                "Code Generator",
                vec!["code", "generation", "ai"],
                vec!["development"],
                "generate code",
                true,
            ),
            (
                "data-parser-1",
                "Data Parser",
                vec!["data", "parsing", "analysis"],
                vec!["data"],
                "parse data",
                true,
            ),
            (
                "email-composer-1",
                "Email Composer",
                vec!["email", "communication"],
                vec!["productivity"],
                "write email",
                true,
            ),
            (
                "file-manager-1",
                "File Manager",
                vec!["file", "filesystem"],
                vec!["system"],
                "manage files",
                true,
            ),
            (
                "web-scraper-1",
                "Web Scraper",
                vec!["web", "scraping", "crawling"],
                vec!["data"],
                "scrape website",
                true,
            ),
            (
                "code-gen-1",
                "Code Generator",
                vec!["code", "generation", "ai"],
                vec!["development"],
                "cook food",
                false,
            ),
            (
                "data-parser-1",
                "Data Parser",
                vec!["data", "parsing", "analysis"],
                vec!["data"],
                "play music",
                false,
            ),
            (
                "email-composer-1",
                "Email Composer",
                vec!["email", "communication"],
                vec!["productivity"],
                "paint picture",
                false,
            ),
        ];

        for (id, name, tags, categories, _, _) in &test_cases {
            let metadata = create_test_metadata(
                id,
                name,
                tags.iter().map(|s| s.to_string()).collect(),
                categories.iter().map(|s| s.to_string()).collect(),
            );
            store.index_metadata(metadata).unwrap();
        }

        let engine = IntentMatchingEngine::new(Arc::new(store), MatchConfig::default());

        let mut correct = 0;
        let total = test_cases.len();

        for (id, _, _, _, intent, should_match) in &test_cases {
            let results = engine.match_intent(intent).unwrap();
            let matched = results.iter().any(|m| m.skill_id == *id);
            if (matched && *should_match) || (!matched && !*should_match) {
                correct += 1;
            }
        }

        let accuracy = correct as f64 / total as f64;
        assert!(accuracy >= 0.85, "Accuracy {} is below 85%", accuracy);
    }

    #[test]
    fn test_with_cache_capacity() {
        let store = InMemoryMetadataIndexStore::new();
        let engine = IntentMatchingEngine::new(Arc::new(store), MatchConfig::default())
            .with_cache_capacity(50);
        assert_eq!(engine.cache_capacity, 50);
    }
}

mod skill_cache_tests {
    use super::*;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.max_size, 100);
        assert!(config.ttl.is_none());
    }

    #[test]
    fn test_cache_config_with_max_size() {
        let config = CacheConfig::default().with_max_size(50);
        assert_eq!(config.max_size, 50);
    }

    #[test]
    fn test_cache_config_with_ttl() {
        let config = CacheConfig::default().with_ttl(Duration::from_secs(60));
        assert!(config.ttl.is_some());
    }

    #[test]
    fn test_cache_config_new() {
        let config = CacheConfig::new(200, Some(Duration::from_secs(30)));
        assert_eq!(config.max_size, 200);
        assert!(config.ttl.is_some());
    }

    #[test]
    fn test_cache_stats_new() {
        let stats = CacheStats::new();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.total_load_time, Duration::from_secs(0));
    }

    #[test]
    fn test_cache_put_and_get() {
        let config = CacheConfig::default();
        let mut cache = SkillLoadCache::new(config);
        let skill = create_test_skill("test-skill-1", "Test Skill", vec![], vec![]);

        cache.put("test-skill-1".to_string(), skill.clone());
        let retrieved = cache.get("test-skill-1");

        assert!(retrieved.is_some());
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn test_cache_miss() {
        let config = CacheConfig::default();
        let mut cache = SkillLoadCache::new(config);

        let retrieved = cache.get("non-existent-skill");

        assert!(retrieved.is_none());
        assert_eq!(cache.stats().misses, 1);
    }

    #[test]
    fn test_cache_eviction() {
        let config = CacheConfig::default().with_max_size(2);
        let mut cache = SkillLoadCache::new(config);

        cache.put(
            "skill-1".to_string(),
            create_test_skill("skill-1", "Skill 1", vec![], vec![]),
        );
        cache.put(
            "skill-2".to_string(),
            create_test_skill("skill-2", "Skill 2", vec![], vec![]),
        );
        cache.put(
            "skill-3".to_string(),
            create_test_skill("skill-3", "Skill 3", vec![], vec![]),
        );

        assert_eq!(cache.stats().evictions, 1);
        assert!(cache.get("skill-1").is_none());
        assert!(cache.get("skill-2").is_some());
        assert!(cache.get("skill-3").is_some());
    }

    #[test]
    fn test_cache_lru_order() {
        let config = CacheConfig::default().with_max_size(3);
        let mut cache = SkillLoadCache::new(config);

        cache.put(
            "skill-1".to_string(),
            create_test_skill("skill-1", "Skill 1", vec![], vec![]),
        );
        cache.put(
            "skill-2".to_string(),
            create_test_skill("skill-2", "Skill 2", vec![], vec![]),
        );
        cache.put(
            "skill-3".to_string(),
            create_test_skill("skill-3", "Skill 3", vec![], vec![]),
        );

        cache.get("skill-1");

        cache.put(
            "skill-4".to_string(),
            create_test_skill("skill-4", "Skill 4", vec![], vec![]),
        );

        assert!(cache.get("skill-2").is_none());
        assert!(cache.get("skill-1").is_some());
        assert!(cache.get("skill-3").is_some());
        assert!(cache.get("skill-4").is_some());
    }

    #[test]
    fn test_cache_remove() {
        let config = CacheConfig::default();
        let mut cache = SkillLoadCache::new(config);

        cache.put(
            "test-skill".to_string(),
            create_test_skill("test-skill", "Test Skill", vec![], vec![]),
        );
        assert!(cache.get("test-skill").is_some());

        cache.remove("test-skill");
        assert!(cache.get("test-skill").is_none());
    }

    #[test]
    fn test_cache_remove_nonexistent() {
        let config = CacheConfig::default();
        let mut cache = SkillLoadCache::new(config);
        cache.remove("nonexistent");
    }

    #[test]
    fn test_cache_clear() {
        let config = CacheConfig::default();
        let mut cache = SkillLoadCache::new(config);

        cache.put(
            "skill-1".to_string(),
            create_test_skill("skill-1", "Skill 1", vec![], vec![]),
        );
        cache.put(
            "skill-2".to_string(),
            create_test_skill("skill-2", "Skill 2", vec![], vec![]),
        );

        cache.clear();

        assert!(cache.get("skill-1").is_none());
        assert!(cache.get("skill-2").is_none());
    }

    #[test]
    fn test_cache_hit_rate() {
        let config = CacheConfig::default();
        let mut cache = SkillLoadCache::new(config);

        cache.put(
            "skill-1".to_string(),
            create_test_skill("skill-1", "Skill 1", vec![], vec![]),
        );

        cache.get("skill-1");
        cache.get("skill-1");
        cache.get("skill-2");

        assert!((cache.hit_rate() - 2.0 / 3.0).abs() < 0.0001);
    }

    #[test]
    fn test_cache_hit_rate_zero() {
        let config = CacheConfig::default();
        let cache = SkillLoadCache::new(config);
        assert_eq!(cache.hit_rate(), 0.0);
    }

    #[test]
    fn test_cache_ttl_expiration() {
        let config = CacheConfig::default().with_ttl(Duration::from_millis(10));
        let mut cache = SkillLoadCache::new(config);

        cache.put(
            "skill-1".to_string(),
            create_test_skill("skill-1", "Skill 1", vec![], vec![]),
        );

        assert!(cache.get("skill-1").is_some());

        std::thread::sleep(Duration::from_millis(20));

        let retrieved = cache.get("skill-1");
        assert!(retrieved.is_none());
        assert_eq!(cache.stats().evictions, 1);
    }

    #[test]
    fn test_cache_debug() {
        let config = CacheConfig::default();
        let cache = SkillLoadCache::new(config);
        let debug_str = format!("{:?}", cache);
        assert!(debug_str.contains("SkillLoadCache"));
    }

    #[test]
    fn test_cache_clone() {
        let config = CacheConfig::default().with_max_size(10);
        let cache = SkillLoadCache::new(config);
        let cloned = cache.clone();
        assert_eq!(cloned.max_size, 10);
    }
}

mod progressive_disclosure_tests {
    use super::*;

    fn setup_registry_with_skills() -> Arc<SkillRegistry> {
        let registry = SkillRegistry::new();

        registry.register(create_test_skill(
            "skill-1",
            "Code Generator",
            vec!["code".to_string()],
            vec!["development".to_string()],
        ));
        registry.register(create_test_skill(
            "skill-2",
            "Data Parser",
            vec!["data".to_string()],
            vec!["data".to_string()],
        ));
        registry.register(create_test_skill(
            "skill-3",
            "Email Composer",
            vec!["email".to_string()],
            vec!["productivity".to_string()],
        ));

        Arc::new(registry)
    }

    #[test]
    fn test_new_manager() {
        let registry = Arc::new(SkillRegistry::new());
        let manager =
            ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::Lazy).unwrap();
        assert_eq!(manager.loading_strategy, LoadingStrategy::Lazy);
    }

    #[test]
    fn test_new_manager_with_eager_metadata() {
        let registry = Arc::new(SkillRegistry::new());
        let manager =
            ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::EagerMetadata)
                .unwrap();
        assert_eq!(manager.loading_strategy, LoadingStrategy::EagerMetadata);
    }

    #[test]
    fn test_new_manager_with_eager_critical() {
        let registry = Arc::new(SkillRegistry::new());
        let manager =
            ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::EagerCritical)
                .unwrap();
        assert_eq!(manager.loading_strategy, LoadingStrategy::EagerCritical);
    }

    #[test]
    fn test_with_skill_cache() {
        let registry = Arc::new(SkillRegistry::new());
        let manager =
            ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::Lazy).unwrap();
        let cache = SkillLoadCache::new(CacheConfig::default().with_max_size(50));
        let manager = manager.with_skill_cache(cache);
    }

    #[test]
    fn test_with_matching_config() {
        let registry = Arc::new(SkillRegistry::new());
        let manager =
            ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::Lazy).unwrap();
        let config = MatchConfig {
            threshold: 0.7,
            max_results: 5,
            enable_tag_match: true,
            enable_category_match: true,
            enable_keyword_match: true,
        };
        let manager = manager.with_matching_config(config);
    }

    #[test]
    fn test_list_indexed_skills_empty() {
        let registry = Arc::new(SkillRegistry::new());
        let manager =
            ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::Lazy).unwrap();
        let skills = manager.list_indexed_skills().unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_load_skill_nonexistent() {
        let registry = Arc::new(SkillRegistry::new());
        let mut manager =
            ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::Lazy).unwrap();
        let result = manager.load_skill("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_or_load_skill_nonexistent() {
        let registry = Arc::new(SkillRegistry::new());
        let mut manager =
            ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::Lazy).unwrap();
        let result = manager.get_or_load_skill("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_clear_cache() {
        let registry = Arc::new(SkillRegistry::new());
        let mut manager =
            ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::Lazy).unwrap();
        manager.clear_cache();
    }

    #[test]
    fn test_manager_debug() {
        let registry = Arc::new(SkillRegistry::new());
        let manager =
            ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::Lazy).unwrap();
        let debug_str = format!("{:?}", manager);
        assert!(debug_str.contains("ProgressiveDisclosureManager"));
    }

    #[test]
    fn test_manager_clone() {
        let registry = setup_registry_with_skills();
        let manager =
            ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::Lazy).unwrap();
        let cloned = manager.clone();
        assert_eq!(cloned.loading_strategy, LoadingStrategy::Lazy);
    }

    #[test]
    fn test_loading_strategy_equality() {
        assert_eq!(LoadingStrategy::Lazy, LoadingStrategy::Lazy);
        assert_eq!(
            LoadingStrategy::EagerMetadata,
            LoadingStrategy::EagerMetadata
        );
        assert_eq!(
            LoadingStrategy::EagerCritical,
            LoadingStrategy::EagerCritical
        );
        assert_ne!(LoadingStrategy::Lazy, LoadingStrategy::EagerMetadata);
    }

    #[test]
    fn test_loading_strategy_debug() {
        let debug_str = format!("{:?}", LoadingStrategy::Lazy);
        assert!(debug_str.contains("Lazy"));
    }

    #[test]
    fn test_registry_with_progressive_disclosure() {
        let registry = SkillRegistry::new();
        let manager = registry
            .with_progressive_disclosure(LoadingStrategy::Lazy)
            .unwrap();
        assert_eq!(manager.loading_strategy, LoadingStrategy::Lazy);
    }

    #[test]
    fn test_find_matching_skills_empty() {
        let registry = Arc::new(SkillRegistry::new());
        let manager =
            ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::Lazy).unwrap();
        let results = manager.find_matching_skills("test").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_skill_registry_backward_compatibility() {
        let registry = SkillRegistry::new();
        let skill = create_test_skill("test-skill", "Test Skill", vec![], vec![]);
        registry.register(skill.clone());

        let retrieved = registry.get("test-skill");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().metadata().id, "test-skill");

        let list = registry.list();
        assert_eq!(list.len(), 1);

        assert!(registry.exists("test-skill"));
    }
}

mod performance_tests {
    use super::*;

    #[test]
    fn test_metadata_index_performance() {
        let mut store = InMemoryMetadataIndexStore::new();
        let count = 1000;

        let start = Instant::now();

        for i in 0..count {
            let metadata = create_test_metadata(
                &format!("skill-{}", i),
                &format!("Skill {}", i),
                vec!["test".to_string(), "performance".to_string()],
                vec!["test".to_string()],
            );
            store.index_metadata(metadata).unwrap();
        }

        let duration = start.elapsed();
        assert!(
            duration < Duration::from_secs(2),
            "Indexing took too long: {:?}",
            duration
        );

        let list_start = Instant::now();
        let all = store.list_all().unwrap();
        let list_duration = list_start.elapsed();
        assert_eq!(all.len(), count);
        assert!(
            list_duration < Duration::from_millis(100),
            "Listing took too long: {:?}",
            list_duration
        );

        let search_start = Instant::now();
        let results = store.search_by_name("skill").unwrap();
        let search_duration = search_start.elapsed();
        assert!(!results.is_empty());
        assert!(
            search_duration < Duration::from_millis(50),
            "Search took too long: {:?}",
            search_duration
        );
    }

    #[test]
    fn test_intent_matching_performance() {
        let mut store = InMemoryMetadataIndexStore::new();
        let count = 1000;

        for i in 0..count {
            let metadata = create_test_metadata(
                &format!("skill-{}", i),
                &format!("Skill {}", i),
                vec!["test".to_string(), format!("tag-{}", i % 10)],
                vec!["category-1".to_string(), format!("category-{}", i % 5)],
            );
            store.index_metadata(metadata).unwrap();
        }

        let engine = IntentMatchingEngine::new(Arc::new(store), MatchConfig::default());

        let start = Instant::now();
        let results = engine.match_intent("test skill").unwrap();
        let duration = start.elapsed();

        assert!(!results.is_empty());
        assert!(
            duration < Duration::from_millis(50),
            "Matching took too long: {:?}",
            duration
        );
    }

    #[test]
    fn test_cache_operations_performance() {
        let config = CacheConfig::default().with_max_size(1000);
        let mut cache = SkillLoadCache::new(config);

        let put_start = Instant::now();
        for i in 0..1000 {
            let skill = create_test_skill(
                &format!("skill-{}", i),
                &format!("Skill {}", i),
                vec![],
                vec![],
            );
            cache.put(format!("skill-{}", i), skill);
        }
        let put_duration = put_start.elapsed();
        assert!(
            put_duration < Duration::from_millis(500),
            "Put operations took too long: {:?}",
            put_duration
        );

        let get_start = Instant::now();
        for i in 0..1000 {
            cache.get(&format!("skill-{}", i));
        }
        let get_duration = get_start.elapsed();
        assert!(
            get_duration < Duration::from_millis(100),
            "Get operations took too long: {:?}",
            get_duration
        );
    }
}

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_metadata_index() {
        let store = InMemoryMetadataIndexStore::new();

        assert!(store.get_metadata("anything").unwrap().is_none());
        assert!(store.search_by_name("anything").unwrap().is_empty());
        assert!(
            store
                .search_by_tags(&["anything".to_string()])
                .unwrap()
                .is_empty()
        );
        assert!(store.search_by_category("anything").unwrap().is_empty());
        assert!(store.list_all().unwrap().is_empty());
    }

    #[test]
    fn test_duplicate_metadata_ids() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata1 = create_test_metadata("test-1", "First Version", vec![], vec![]);
        let metadata2 = create_test_metadata("test-1", "Second Version", vec![], vec![]);

        store.index_metadata(metadata1).unwrap();
        store.index_metadata(metadata2).unwrap();

        let retrieved = store.get_metadata("test-1").unwrap().unwrap();
        assert_eq!(retrieved.name, "Second Version");
    }

    #[test]
    fn test_special_characters_in_search() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata = create_test_metadata(
            "test-1",
            "Skill!@#$%^&*()",
            vec!["tag!@#".to_string()],
            vec!["cat!@#".to_string()],
        );

        store.index_metadata(metadata).unwrap();

        let results = store.search_by_name("Skill!").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_unicode_in_metadata() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata = create_test_metadata(
            "test-1",
            "技能",
            vec!["标签".to_string()],
            vec!["分类".to_string()],
        );

        store.index_metadata(metadata).unwrap();

        let retrieved = store.get_metadata("test-1").unwrap().unwrap();
        assert_eq!(retrieved.name, "技能");

        let results = store.search_by_name("技能").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_extremely_long_names() {
        let mut store = InMemoryMetadataIndexStore::new();
        let long_name = "a".repeat(10000);
        let metadata = create_test_metadata("test-1", &long_name, vec![], vec![]);

        store.index_metadata(metadata).unwrap();

        let retrieved = store.get_metadata("test-1").unwrap().unwrap();
        assert_eq!(retrieved.name.len(), 10000);
    }

    #[test]
    fn test_many_tags_and_categories() {
        let mut store = InMemoryMetadataIndexStore::new();
        let tags: Vec<String> = (0..100).map(|i| format!("tag-{}", i)).collect();
        let categories: Vec<String> = (0..50).map(|i| format!("cat-{}", i)).collect();

        let metadata = create_test_metadata("test-1", "Test Skill", tags, categories);
        store.index_metadata(metadata).unwrap();

        let results = store.search_by_tags(&["tag-50".to_string()]).unwrap();
        assert_eq!(results.len(), 1);

        let results = store.search_by_category("cat-25").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_empty_tags_and_categories() {
        let mut store = InMemoryMetadataIndexStore::new();
        let metadata = create_test_metadata("test-1", "Test Skill", vec![], vec![]);

        store.index_metadata(metadata).unwrap();

        let results = store.search_by_tags(&["test".to_string()]).unwrap();
        assert!(results.is_empty());

        let results = store.search_by_category("test").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_intent_matching_with_no_skills() {
        let store = InMemoryMetadataIndexStore::new();
        let engine = IntentMatchingEngine::new(Arc::new(store), MatchConfig::default());

        let results = engine.match_intent("test query").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_cache_with_zero_size() {
        let config = CacheConfig::default().with_max_size(0);
        let mut cache = SkillLoadCache::new(config);

        let skill = create_test_skill("test-1", "Test Skill", vec![], vec![]);
        cache.put("test-1".to_string(), skill);

        assert_eq!(cache.stats().evictions, 1);
        assert!(cache.get("test-1").is_none());
    }

    #[test]
    fn test_cache_with_very_large_size() {
        let config = CacheConfig::default().with_max_size(1_000_000);
        let mut cache = SkillLoadCache::new(config);

        for i in 0..1000 {
            let skill = create_test_skill(
                &format!("skill-{}", i),
                &format!("Skill {}", i),
                vec![],
                vec![],
            );
            cache.put(format!("skill-{}", i), skill);
        }

        assert_eq!(cache.stats().evictions, 0);
    }
}
