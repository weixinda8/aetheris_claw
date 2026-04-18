use crate::skill::{
    InMemoryMetadataIndexStore, IntentMatchingEngine, MatchConfig, Skill, SkillLoadCache,
    SkillLoader, SkillMatch, SkillMetadata, SkillRegistry,
    metadata_index::MetadataIndexStore,
};
use crate::utils::{AetherisError, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadingStrategy {
    Lazy,
    EagerMetadata,
    EagerCritical,
}

pub struct ProgressiveDisclosureManager {
    registry: Arc<SkillRegistry>,
    index_store: InMemoryMetadataIndexStore,
    matching_engine: IntentMatchingEngine,
    skill_cache: SkillLoadCache,
    loading_strategy: LoadingStrategy,
    fallback_to_full_load: bool,
}

impl Clone for ProgressiveDisclosureManager {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            index_store: self.index_store.clone(),
            matching_engine: self.matching_engine.clone(),
            skill_cache: self.skill_cache.clone(),
            loading_strategy: self.loading_strategy.clone(),
            fallback_to_full_load: self.fallback_to_full_load,
        }
    }
}

impl std::fmt::Debug for ProgressiveDisclosureManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressiveDisclosureManager")
            .field("registry", &"Arc<SkillRegistry>")
            .field("index_store", &self.index_store)
            .field("matching_engine", &"IntentMatchingEngine")
            .field("skill_cache", &self.skill_cache)
            .field("loading_strategy", &self.loading_strategy)
            .field("fallback_to_full_load", &self.fallback_to_full_load)
            .finish()
    }
}

impl ProgressiveDisclosureManager {
    pub fn new(registry: Arc<SkillRegistry>, loading_strategy: LoadingStrategy) -> Result<Self> {
        info!(
            "Creating ProgressiveDisclosureManager with strategy: {:?}",
            loading_strategy
        );

        let index_store = InMemoryMetadataIndexStore::new();
        let matching_engine =
            IntentMatchingEngine::new(Arc::new(index_store.clone()), MatchConfig::default());
        let skill_cache = SkillLoadCache::new(Default::default());

        Ok(Self {
            registry,
            index_store,
            matching_engine,
            skill_cache,
            loading_strategy,
            fallback_to_full_load: true,
        })
    }

    pub fn with_skill_cache(mut self, cache: SkillLoadCache) -> Self {
        debug!("Configuring SkillLoadCache");
        self.skill_cache = cache;
        self
    }

    pub fn with_matching_config(mut self, config: MatchConfig) -> Self {
        debug!("Configuring MatchConfig");
        self.matching_engine =
            IntentMatchingEngine::new(Arc::new(self.index_store.clone()), config);
        self
    }

    #[instrument(skip(self, paths))]
    pub fn index_skills(&mut self, paths: &[PathBuf]) -> Result<usize> {
        info!("Indexing skills from {} paths", paths.len());

        let mut count = 0;

        for path in paths {
            debug!("Processing path: {:?}", path);

            let loader = SkillLoader::new();
            let skills_result = tokio::runtime::Runtime::new()
                .map_err(|e| {
                    AetherisError::Skill(format!("Failed to create tokio runtime: {}", e))
                })?
                .block_on(loader.load_from_path(path.to_str().unwrap_or("")));

            if let Ok(skills) = skills_result {
                for skill in skills {
                    let metadata = skill.metadata().clone();
                    let skill_id = metadata.id.clone();

                    debug!("Indexing skill: {}", skill_id);

                    self.index_store.index_metadata(metadata)?;

                    if (self.loading_strategy == LoadingStrategy::EagerCritical
                        || self.loading_strategy == LoadingStrategy::EagerMetadata)
                        && skill.metadata().priority.should_preload() {
                            info!("Preloading critical skill: {}", skill_id);
                            self.registry.register(skill.clone());
                            self.skill_cache.put(skill_id, skill);
                        }

                    count += 1;
                }
            } else {
                warn!("Failed to load skills from path: {:?}", path);
                if self.fallback_to_full_load {
                    debug!("Fallback to full load for path: {:?}", path);
                }
            }
        }

        info!("Successfully indexed {} skills", count);
        Ok(count)
    }

    #[instrument(skip(self, intent))]
    pub fn find_matching_skills(&self, intent: &str) -> Result<Vec<SkillMatch>> {
        debug!("Finding matching skills for intent: {}", intent);

        let matches = self.matching_engine.match_intent(intent)?;

        info!("Found {} matching skills for intent", matches.len());
        Ok(matches)
    }

    #[instrument(skip(self, skill_id))]
    pub fn load_skill(&mut self, skill_id: &str) -> Result<Arc<dyn Skill>> {
        info!("Loading skill: {}", skill_id);

        let skill = self.registry.get(skill_id).ok_or_else(|| {
            if self.fallback_to_full_load {
                warn!(
                    "Skill not found in registry, attempting full load fallback: {}",
                    skill_id
                );
            }
            AetherisError::Skill(format!("Skill not found: {}", skill_id))
        })?;

        self.skill_cache.put(skill_id.to_string(), skill.clone());

        info!("Successfully loaded and cached skill: {}", skill_id);
        Ok(skill)
    }

    #[instrument(skip(self, skill_id))]
    pub fn get_or_load_skill(&mut self, skill_id: &str) -> Result<Arc<dyn Skill>> {
        debug!("Getting or loading skill: {}", skill_id);

        if let Some(cached) = self.skill_cache.get(skill_id) {
            debug!("Cache hit for skill: {}", skill_id);
            return Ok(cached);
        }

        debug!("Cache miss, loading skill: {}", skill_id);
        self.load_skill(skill_id)
    }

    #[instrument(skip(self))]
    pub fn list_indexed_skills(&self) -> Result<Vec<SkillMetadata>> {
        debug!("Listing indexed skills");

        let skills = self.index_store.list_all()?;

        info!("Listed {} indexed skills", skills.len());
        Ok(skills)
    }

    #[instrument(skip(self))]
    pub fn clear_cache(&mut self) {
        info!("Clearing skill cache");
        self.skill_cache.clear();
    }
}
